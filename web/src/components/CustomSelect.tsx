import { useId } from "react";
import { createPortal } from "react-dom";
import { Icon } from "./Icon";
import {
  useSelectInteraction,
  type SelectOption,
} from "./useSelectInteraction";
import { useSelectPosition } from "./useSelectPosition";
import "./custom-select.css";

interface Props {
  label: string;
  value: string;
  options: SelectOption[];
  onChange: (value: string) => void;
  placeholder?: string;
  disabled?: boolean;
}

/** Shared, select-only combobox for viewer scope and preview controls. */
export function CustomSelect({
  label,
  value,
  options,
  onChange,
  placeholder = "Choose…",
  disabled = false,
}: Props) {
  const id = useId();
  const unavailable = disabled || !options.some((option) => !option.disabled);
  const ui = useSelectInteraction(options, value, unavailable, onChange);
  const position = useSelectPosition(ui.trigger, ui.open);
  const selected = options.find((option) => option.value === value);
  const activeId = ui.active >= 0 ? `${id}-option-${ui.active}` : undefined;
  return (
    <>
      <button
        ref={ui.trigger}
        type="button"
        className="custom-select"
        role="combobox"
        aria-label={label}
        aria-haspopup="listbox"
        aria-expanded={ui.open}
        aria-controls={ui.open ? `${id}-listbox` : undefined}
        aria-activedescendant={ui.open ? activeId : undefined}
        disabled={unavailable}
        value={value}
        title={selected?.label ?? placeholder}
        onKeyDown={ui.onKeyDown}
        onClick={() => (ui.open ? ui.close() : ui.show())}
      >
        {selected?.activity && (
          <span className="select-activity" aria-label="New changes" />
        )}
        <span className="select-value">{selected?.label ?? placeholder}</span>
        {selected?.description && (
          <span className="select-detail">{selected.description}</span>
        )}
        <span className="select-chevron">
          <Icon name="chevron" size={12} />
        </span>
      </button>
      {ui.open &&
        createPortal(
          <div
            ref={ui.menu}
            id={`${id}-listbox`}
            role="listbox"
            aria-label={label}
            className="select-menu"
            style={position}
          >
            <div className="select-menu-label" aria-hidden="true">
              {label}
            </div>
            {options.map((option, index) => (
              <div
                key={option.value}
                id={`${id}-option-${index}`}
                role="option"
                aria-selected={option.value === value}
                aria-disabled={option.disabled || undefined}
                aria-label={`${option.label}${option.activity ? ", new changes" : ""}${option.description ? `, ${option.description}` : ""}`}
                data-value={option.value}
                data-index={index}
                className={`select-option ${index === ui.active ? "is-active" : ""}`}
                onPointerMove={() => !option.disabled && ui.setActive(index)}
                onPointerDown={(event) => event.preventDefault()}
                onClick={() => ui.choose(index)}
              >
                <span className="select-check" aria-hidden="true">
                  {option.value === value ? "✓" : ""}
                </span>
                <span className="select-option-copy">
                  <span>{option.label}</span>
                  {option.description && <small>{option.description}</small>}
                </span>
                {option.activity && (
                  <span className="select-activity" aria-hidden="true" />
                )}
              </div>
            ))}
          </div>,
          document.body,
        )}
    </>
  );
}
