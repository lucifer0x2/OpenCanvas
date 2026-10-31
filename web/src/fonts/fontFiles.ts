const MAX_FONT_BYTES = 10 * 1024 * 1024;
const FORMATS = {
  ttf: { mediaType: "font/ttf", signatures: ["00010000", "74727565"] },
  otf: { mediaType: "font/otf", signatures: ["4f54544f"] },
  woff2: { mediaType: "font/woff2", signatures: ["774f4632"] },
};

/** Check extension, bounded bytes and file signature before encoding the actual file. */
export async function readFontFile(file: File) {
  const extension = file.name.split(".").pop()?.toLowerCase();
  const format =
    extension && Object.hasOwn(FORMATS, extension)
      ? FORMATS[extension as keyof typeof FORMATS]
      : undefined;
  if (!format) throw new Error("Choose a TTF, OTF or WOFF2 font file.");
  if (!file.size) throw new Error("This font file is empty.");
  if (file.size > MAX_FONT_BYTES)
    throw new Error("Font files must be 10 MiB or smaller.");
  const bytes = new Uint8Array(await file.arrayBuffer());
  const signature = Array.from(bytes.slice(0, 4), (byte) =>
    byte.toString(16).padStart(2, "0"),
  ).join("");
  if (!format.signatures.includes(signature))
    throw new Error("The file contents do not match its font format.");
  let binary = "";
  for (let offset = 0; offset < bytes.length; offset += 8192)
    binary += String.fromCharCode(...bytes.subarray(offset, offset + 8192));
  return { mediaType: format.mediaType, base64: btoa(binary) };
}
