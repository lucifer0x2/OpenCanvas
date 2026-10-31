export interface ProjectFont {
  id: string;
  family: string;
  weight: number;
  style: string;
  source: "project" | "bundled";
  status: string;
  license?: string;
  mediaType?: string;
  byteSize?: number;
  downloadUrl?: string;
}

export interface FontList {
  projectId: string;
  revision: number;
  fonts: ProjectFont[];
}

export interface FontDetails {
  family: string;
  weight: number;
  style: string;
  license: string;
}

export interface FontImport {
  expectedRevision: number;
  idempotencyKey: string;
  mediaType: string;
  base64: string;
  font: {
    id: string;
    family: string;
    weight: number;
    style: string;
    license?: string;
  };
}
