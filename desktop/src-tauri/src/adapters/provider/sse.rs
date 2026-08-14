#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SseFrame {
    pub event: Option<String>,
    pub data: String,
    pub id: Option<String>,
    pub retry_ms: Option<u64>,
}

#[derive(Debug, Default)]
pub struct SseDecoder {
    buffer: String,
}

impl SseDecoder {
    pub fn push(&mut self, chunk: &[u8]) -> Result<Vec<SseFrame>, std::str::Utf8Error> {
        self.buffer.push_str(std::str::from_utf8(chunk)?);
        self.buffer = self.buffer.replace("\r\n", "\n");
        let mut frames = Vec::new();

        while let Some(boundary) = self.buffer.find("\n\n") {
            let block = self.buffer[..boundary].to_string();
            self.buffer.drain(..boundary + 2);
            if let Some(frame) = parse_block(&block) {
                frames.push(frame);
            }
        }
        Ok(frames)
    }

    pub fn finish(&mut self) -> Option<SseFrame> {
        let block = std::mem::take(&mut self.buffer);
        parse_block(block.trim_end_matches(['\r', '\n']))
    }
}

fn parse_block(block: &str) -> Option<SseFrame> {
    let mut event = None;
    let mut data = Vec::new();
    let mut id = None;
    let mut retry_ms = None;
    for line in block.lines() {
        if line.is_empty() || line.starts_with(':') {
            continue;
        }
        let (field, value) = line.split_once(':').unwrap_or((line, ""));
        let value = value.strip_prefix(' ').unwrap_or(value);
        match field {
            "event" => event = Some(value.to_string()),
            "data" => data.push(value.to_string()),
            "id" if !value.contains('\0') => id = Some(value.to_string()),
            "retry" => retry_ms = value.parse().ok(),
            _ => {}
        }
    }
    (!data.is_empty()).then(|| SseFrame {
        event,
        data: data.join("\n"),
        id,
        retry_ms,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_half_packets_multiline_data_and_heartbeats() {
        let mut decoder = SseDecoder::default();
        assert!(
            decoder
                .push(b": keep-alive\r\n\r\nevent: mes")
                .unwrap()
                .is_empty()
        );
        let frames = decoder
            .push(b"sage\r\nid: 7\r\ndata: first\r\ndata: second\r\n\r\n")
            .unwrap();
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].event.as_deref(), Some("message"));
        assert_eq!(frames[0].id.as_deref(), Some("7"));
        assert_eq!(frames[0].data, "first\nsecond");
    }

    #[test]
    fn flushes_a_final_frame_without_blank_terminator() {
        let mut decoder = SseDecoder::default();
        decoder.push(b"data: [DONE]").unwrap();
        assert_eq!(decoder.finish().unwrap().data, "[DONE]");
    }
}
