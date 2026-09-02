export type SemanticModelPackStatus =
  | {
      state: "missing";
      modelVersion: string;
      missingFiles: string[];
    }
  | {
      state: "corrupt";
      modelVersion: string;
      invalidFiles: string[];
    }
  | {
      state: "ready";
      modelVersion: string;
      dimensions: number;
      totalBytes: number;
    };
