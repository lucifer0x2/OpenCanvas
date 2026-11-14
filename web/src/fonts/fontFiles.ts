const MAX_FONT_BYTES = 10 * 1024 * 1024;
const FORMATS = {
  ttf: { mediaType: "font/ttf", signatures: ["00010000", "74727565"] },
  otf: { mediaType: "font/otf", signatures: ["4f54544f"] },
  woff2: { mediaType: "font/woff2", signatures: ["774f4632"] },
};

function fontFormat(file: File) {
  const extension = file.name.split(".").pop()?.toLowerCase();
  const format =
    extension && Object.hasOwn(FORMATS, extension)
      ? FORMATS[extension as keyof typeof FORMATS]
      : undefined;
  if (!format) throw new Error("Choose a TTF, OTF or WOFF2 font file.");
  if (!file.size) throw new Error("This font file is empty.");
  if (file.size > MAX_FONT_BYTES)
    throw new Error("Font files must be 10 MiB or smaller.");
  return format;
}

function checkSignature(
  bytes: Uint8Array,
  format: (typeof FORMATS)[keyof typeof FORMATS],
) {
  const signature = Array.from(bytes.slice(0, 4), (byte) =>
    byte.toString(16).padStart(2, "0"),
  ).join("");
  if (!format.signatures.includes(signature))
    throw new Error("The file contents do not match its font format.");
}

/** Selection checks only the header; full bytes and base64 are needed only on submit. */
export async function validateFontFile(file: File) {
  const format = fontFormat(file);
  checkSignature(new Uint8Array(await file.slice(0, 4).arrayBuffer()), format);
}

/** Recheck the actual submitted bytes before encoding the bounded file. */
export async function readFontFile(file: File) {
  const format = fontFormat(file);
  const bytes = new Uint8Array(await file.arrayBuffer());
  checkSignature(bytes, format);
  let binary = "";
  for (let offset = 0; offset < bytes.length; offset += 8192)
    binary += String.fromCharCode(...bytes.subarray(offset, offset + 8192));
  return { mediaType: format.mediaType, base64: btoa(binary) };
}
