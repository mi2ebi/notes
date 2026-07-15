function color(id, tokens) {
  const el = document.getElementById(id);
  if (!el) {
    console.error(`color(): no element with id ${JSON.stringify(id)}`);
    return;
  }
  const actual = el.textContent;
  const expected = tokens.map(([text]) => text).join("");
  if (actual !== expected) {
    console.error(
      `color(): #${id}'s content doesn't match its saved highlighting - ` +
        "rerun the highlighter on this block.",
    );
    const warning = document.createElement("div");
    warning.textContent = `\u26a0 highlighting is stale for #${id} - rerun the tool on this block.`;
    warning.style.cssText =
      "color:#fff;background:#a33;padding:2px 6px;font:12px monospace;margin-bottom:2px;";
    el.parentNode.insertBefore(warning, el);
    return;
  }
  el.textContent = "";
  for (const [text, className] of tokens) {
    if (className) {
      const span = document.createElement("span");
      span.className = className;
      span.textContent = text;
      el.appendChild(span);
    } else {
      el.appendChild(document.createTextNode(text));
    }
  }
}
