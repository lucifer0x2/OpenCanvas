// Focused LTR script/mark cases: no font-specific strings or fallback exclusions.
export const scriptCases = {
  greekFirst: "ς i̇", latinFirst: "i̇ ς", adjacent: "ςi̇αj̇Жắ",
  latinOnly: "i̇j̇ ắ", greekOnly: "ά ς ῶ", cyrillic: "Ж i̇ Ж́",
  spaces: "ς   i̇   α", leading: " . (i̇) α", brackets: "α(i̇)ς",
  nested: "α[(i̇)]ς", punctuation: "α—i̇,ς.i̇", numbers: "α 123 i̇ 456 ς",
  inheritedLeading: "̇i̇ α", inheritedOnly: "̇́", neutralOnly: " . () 123",
  stacked: "ς ắ i̇́ ἄ", crossScriptMark: "α a҃ α", markOnSpace: "α ̇ i̇",
  nonbreaking: "α i̇ ς", invisible: "ς\u2060i̇\u200bα\ufeffj̇",
  separators: "α\u2028i̇\u2029ς", joiner: "α\u200di̇\u200cς",
  kerning: "AVαToςffi i̇", precomposed: "άaαái̇", astral: "α𝟙i̇ς",
  stress: "αi̇Жắςj̇ ".repeat(24),
};
