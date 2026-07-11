// src/utils/ime.ts

/**
 * 判断一个键盘事件是否来自 IME（输入法）组合过程。
 *
 * 用于守卫 Enter 等提交类按键：中文/日文等输入法弹出候选词列表时，
 * 用户按 Enter 是在选词，不是在提交表单。
 *
 * 为什么要同时检查两个条件：
 *
 * 1. `e.isComposing`（W3C UI Events 规范标准属性）
 *    - 语义清晰：事件发生在 composition session 期间即为 true。
 *    - Chrome / Firefox 上可靠。
 *    - **WebKit（macOS 系统输入法）不可靠**：候选词确认（选词）的那次
 *      keydown 的 `isComposing` 是 `false`——WebKit 把"确认"视为组合已结束。
 *      这导致 macOS 上中文输入法选词的 Enter 会穿透 `isComposing` 守卫。
 *
 * 2. `e.keyCode === 229`（W3C 为 IME 保留的特殊值）
 *    - 229 表示"此按键正在被输入法处理，不应当作普通字符"。
 *    - 这是 WebKit 上检测 IME 事件的唯一可靠信号
 *      （参考 WebKit bug #165004）。
 *    - `keyCode` 虽被 MDN 标记 deprecated，但在 IME 检测这个用途上
 *      没有现代替代品——`isComposing` 本应替代它，却因 WebKit 的实现
 *      缺陷而无法独立承担。
 *
 * 两者用 `||` 连接：`isComposing` 覆盖 Chrome/Firefox，`keyCode === 229`
 * 补上 WebKit 的缺口。
 *
 * 参考：
 * - https://developer.mozilla.org/en-US/docs/Web/API/KeyboardEvent/isComposing
 * - https://bugs.webkit.org/show_bug.cgi?id=165004
 * - https://www.w3.org/TR/uievents/#keys-keyCode-keyLocation
 */
export function isIMEEvent(e: KeyboardEvent): boolean {
  return e.isComposing || e.keyCode === 229;
}
