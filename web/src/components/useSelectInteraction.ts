import { useEffect, useRef, useState, type KeyboardEvent } from "react";

export interface SelectOption {
  value: string;
  label: string;
  description?: string;
  activity?: boolean;
  disabled?: boolean;
}

/** Focus stays on the combobox; the listbox exposes its active descendant. */
export function useSelectInteraction(
  options: SelectOption[],
  value: string,
  disabled: boolean,
  onChange: (value: string) => void,
) {
  const trigger = useRef<HTMLButtonElement>(null);
  const menu = useRef<HTMLDivElement>(null);
  const [open, setOpen] = useState(false);
  const [active, setActive] = useState(-1);
  const search = useRef({ text: "", time: 0 });
  const enabled = options.flatMap((option, index) =>
    option.disabled ? [] : [index],
  );
  const selected = options.findIndex((option) => option.value === value);
  const show = (index = selected) => {
    if (disabled) return;
    search.current = { text: "", time: 0 };
    setActive(enabled.includes(index) ? index : (enabled[0] ?? -1));
    setOpen(true);
  };
  const choose = (index: number) => {
    if (!options[index] || options[index].disabled) return;
    onChange(options[index].value);
    setOpen(false);
  };

  useEffect(() => {
    if (disabled) setOpen(false);
  }, [disabled]);
  useEffect(() => {
    if (!open) return;
    const outside = (event: PointerEvent) => {
      const target = event.target as Node;
      if (!trigger.current?.contains(target) && !menu.current?.contains(target))
        setOpen(false);
    };
    const blur = (event: FocusEvent) => {
      if (!trigger.current?.contains(event.target as Node)) setOpen(false);
    };
    document.addEventListener("pointerdown", outside);
    document.addEventListener("focusin", blur);
    return () => {
      document.removeEventListener("pointerdown", outside);
      document.removeEventListener("focusin", blur);
    };
  }, [open]);
  useEffect(() => {
    if (open)
      menu.current
        ?.querySelector(`[data-index="${active}"]`)
        ?.scrollIntoView({ block: "nearest" });
  }, [active, open]);

  const onKeyDown = (event: KeyboardEvent<HTMLButtonElement>) => {
    const key = event.key;
    if (key === "Tab") {
      setOpen(false);
      return;
    }
    if (key === "Escape") {
      if (open) {
        event.preventDefault();
        event.stopPropagation();
        setOpen(false);
      }
      return;
    }
    if (event.altKey || event.ctrlKey || event.metaKey || disabled) return;
    if (["ArrowDown", "ArrowUp", "Home", "End", "Enter", " "].includes(key)) {
      event.preventDefault();
      event.stopPropagation();
      if (key === "Enter" || key === " ") {
        if (open) choose(active);
        else show();
      } else if (key === "Home" || key === "End") {
        const index = key === "Home" ? enabled[0] : enabled.at(-1);
        if (open) setActive(index ?? -1);
        else show(index);
      } else if (!open) show();
      else {
        const offset = key === "ArrowDown" ? 1 : -1;
        const current = enabled.indexOf(active);
        setActive(
          enabled[(current + offset + enabled.length) % enabled.length] ?? -1,
        );
      }
      return;
    }
    if (key.length !== 1 || !key.trim()) return;
    event.preventDefault();
    event.stopPropagation();
    const now = Date.now();
    const previous = now - search.current.time < 700 ? search.current.text : "";
    const text = previous + key.toLocaleLowerCase();
    search.current = { text, time: now };
    const query = [...text].every((letter) => letter === text[0])
      ? text[0]
      : text;
    const start = open ? active : selected;
    const ordered = [
      ...enabled.filter((index) => index > start),
      ...enabled.filter((index) => index <= start),
    ];
    const match = ordered.find((index) =>
      options[index].label.toLocaleLowerCase().startsWith(query),
    );
    if (!open) {
      show(match);
      search.current = { text, time: now };
    } else if (match !== undefined) setActive(match);
  };
  return {
    trigger,
    menu,
    open,
    active,
    setActive,
    show,
    close: () => setOpen(false),
    choose,
    onKeyDown,
  };
}
