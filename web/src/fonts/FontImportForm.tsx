import { useRef, useState } from "react";
import { CustomSelect } from "../components/CustomSelect";
import { readFontFile } from "./fontFiles";
import type { FontDetails } from "./types";

interface Props {
  busy: boolean;
  ready: boolean;
  retryable: boolean;
  submit: (file?: File, details?: FontDetails) => Promise<boolean>;
}

export function FontImportForm({ busy, ready, retryable, submit }: Props) {
  const [file, setFile] = useState<File>();
  const [family, setFamily] = useState("");
  const [weight, setWeight] = useState("400");
  const [style, setStyle] = useState("normal");
  const [license, setLicense] = useState("");
  const [error, setError] = useState("");
  const [checking, setChecking] = useState(false);
  const input = useRef<HTMLInputElement>(null);
  const attempt = useRef(0);
  const locked = busy || retryable;
  return (
    <form
      className="font-import"
      onSubmit={async (event) => {
        event.preventDefault();
        if (!retryable && (!file || !family.trim())) {
          setError("Choose a font file and enter its family name.");
          return;
        }
        if (
          !retryable &&
          new TextEncoder().encode(family.trim()).length > 128
        ) {
          setError(
            "Font family must be 128 UTF-8 bytes or fewer. Shorten the family name.",
          );
          return;
        }
        setError("");
        if (
          await submit(file, { family, weight: Number(weight), style, license })
        ) {
          setFile(undefined);
          setFamily("");
          setLicense("");
          if (input.current) input.current.value = "";
        }
      }}
    >
      <h3>Add a font</h3>
      <p className="subtle">
        Import a font you have permission to use. The original file stays with
        this project.
      </p>
      <fieldset disabled={locked}>
        <label className="font-file">
          Font file
          <input
            ref={input}
            type="file"
            accept=".ttf,.otf,.woff2"
            aria-describedby="font-file-help font-file-error"
            onChange={async (event) => {
              const selected = event.currentTarget.files?.[0];
              const version = ++attempt.current;
              setFile(undefined);
              setError("");
              if (!selected) {
                setChecking(false);
                return;
              }
              setChecking(true);
              try {
                await readFontFile(selected);
                if (version === attempt.current) setFile(selected);
              } catch (cause) {
                if (version === attempt.current)
                  setError((cause as Error).message);
              } finally {
                if (version === attempt.current) setChecking(false);
              }
            }}
          />
        </label>
        <p id="font-file-help" className="subtle">
          TTF, OTF or WOFF2 · maximum 10 MiB
        </p>
        <label>
          Font family
          <input
            value={family}
            required
            maxLength={128}
            placeholder="e.g. Noto Sans"
            onChange={(event) => setFamily(event.target.value)}
          />
        </label>
        <div className="font-fields">
          <label>
            Weight
            <CustomSelect
              label="Font weight"
              value={weight}
              disabled={locked}
              onChange={setWeight}
              options={Array.from({ length: 9 }, (_, index) => ({
                value: String((index + 1) * 100),
                label: `${(index + 1) * 100}${index === 3 ? " · Regular" : index === 6 ? " · Bold" : ""}`,
              }))}
            />
          </label>
          <label>
            Style
            <CustomSelect
              label="Font style"
              value={style}
              disabled={locked}
              onChange={setStyle}
              options={[
                { value: "normal", label: "Normal" },
                { value: "italic", label: "Italic" },
              ]}
            />
          </label>
        </div>
        <label>
          License <span className="subtle">(optional)</span>
          <input
            value={license}
            maxLength={1000}
            placeholder="License name or source URL"
            onChange={(event) => setLicense(event.target.value)}
          />
        </label>
      </fieldset>
      {error && (
        <p id="font-file-error" className="font-error" role="alert">
          {error}
        </p>
      )}
      <div className="font-submit">
        <button
          type="submit"
          className="font-primary"
          disabled={
            busy ||
            checking ||
            (!retryable && (!ready || !file || !family.trim()))
          }
        >
          {busy
            ? "Adding font…"
            : retryable
              ? "Retry same import"
              : checking
                ? "Checking file…"
                : "Add font"}
        </button>
        <span className="subtle">
          Adds a resource; existing text keeps its font.
        </span>
      </div>
    </form>
  );
}
