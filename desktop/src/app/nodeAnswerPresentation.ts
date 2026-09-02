import type { RunState } from "../domain/common.ts";

export type MissingNodeAnswerPresentation = {
  message: string;
  showSpinner: boolean;
};

export function presentMissingNodeAnswer(
  runState: RunState,
): MissingNodeAnswerPresentation {
  switch (runState) {
    case "pending":
    case "streaming":
      return { message: "正在等待模型响应", showSpinner: true };
    case "completed":
      return { message: "本次运行已完成，但模型未返回可显示内容。", showSpinner: false };
    case "cancelled":
      return { message: "本次运行已停止，未收到可保留内容。", showSpinner: false };
    case "failed":
      return { message: "本次运行未完成，未收到可保留内容。", showSpinner: false };
  }
}
