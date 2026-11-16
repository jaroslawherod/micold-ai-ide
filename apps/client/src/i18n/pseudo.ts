// The test pseudo-locale (US5, R9): computed from the English catalog, never shipped as a file.
// Letters get accents, the text is padded by 40 percent and bracketed, so a hard-coded string, a
// clipped label or a concatenated sentence shows at once. ICU syntax (arguments, plural keywords)
// is kept intact so the result still formats.
import {
  TYPE,
  parse,
  type MessageFormatElement,
} from "@formatjs/icu-messageformat-parser";
import { printAST } from "@formatjs/icu-messageformat-parser/printer.js";

const FROM = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
const TO = "àƀçđéƒĝĥíĵķłɱñóþɋřšţúṽẁӽýžÀßÇĐÉFĜĤÍĴĶĿMÑÓÞQŘŠŢÚṼŴẊÝŽ";

function accent(text: string): string {
  let out = "";
  for (const ch of text) {
    const i = FROM.indexOf(ch);
    out += i < 0 ? ch : TO[i];
  }
  return out;
}

function transform(elements: MessageFormatElement[]): MessageFormatElement[] {
  return elements.map((el): MessageFormatElement => {
    if (el.type === TYPE.literal) return { ...el, value: accent(el.value) };
    if (el.type === TYPE.tag) return { ...el, children: transform(el.children) };
    if (el.type === TYPE.plural || el.type === TYPE.select) {
      const options = Object.fromEntries(
        Object.entries(el.options).map(([key, option]) => [key, { value: transform(option.value) }]),
      );
      return { ...el, options } as MessageFormatElement;
    }
    return el;
  });
}

function visibleLength(elements: MessageFormatElement[]): number {
  return elements.reduce((n, el) => (el.type === TYPE.literal ? n + el.value.length : n), 0);
}

export function pseudoLocalize(message: string): string {
  const ast = parse(message);
  const padding = "·".repeat(Math.ceil(visibleLength(ast) * 0.4));
  const wrapped: MessageFormatElement[] = [
    { type: TYPE.literal, value: "[" },
    ...transform(ast),
    { type: TYPE.literal, value: `${padding}]` },
  ];
  return printAST(wrapped);
}

export function pseudoCatalog(source: Readonly<Record<string, string>>): Record<string, string> {
  return Object.fromEntries(Object.entries(source).map(([id, m]) => [id, pseudoLocalize(m)]));
}
