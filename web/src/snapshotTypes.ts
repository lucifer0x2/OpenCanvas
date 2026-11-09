/** Complete authored collections consumed by the shared Rust scene compiler. */
export interface SnapshotToken {
  id: string;
  name: string;
  type: string;
  value: unknown;
}

export interface SnapshotStyle {
  id: string;
  name: string;
  kind: string;
  value: unknown;
}

export interface SnapshotAsset {
  id: string;
  contentHash: string;
  mediaType: string;
  byteSize: number;
  relativeKey: string;
  width: number | null;
  height: number | null;
  status: string;
}

export interface SnapshotFont {
  id: string;
  family: string;
  weight: number;
  style: string;
  assetId: string | null;
  bundledId: string | null;
  contentHash: string | null;
  license: string | null;
  status: string;
}
