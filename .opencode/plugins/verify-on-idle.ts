import type { Plugin } from "@opencode-ai/plugin"

/**
 * Stop-hook 等价物（迁移自 .claude/settings.json）。
 *
 * 会话进入 idle（助手回合结束）时，跑与旧 Claude Stop hook 相同的校验：
 *   pnpm vue-tsc --noEmit && (cd src-tauri && cargo check && cargo test)
 *
 * 与 Claude Stop hook 的差异：OpenCode 的 event hook 无法阻塞模型，只能在
 * 回合结束后运行并通过 toast + 日志回报结果。失败时弹 error toast，成功时静默。
 */
export const VerifyOnIdle: Plugin = async ({ client, $, directory }) => {
  let running = false

  const steps: Array<{ label: string; cmd: string; cwd: string }> = [
    { label: "vue-tsc", cmd: "pnpm vue-tsc --noEmit", cwd: directory },
    { label: "cargo check", cmd: "cargo check", cwd: `${directory}/src-tauri` },
    { label: "cargo test", cmd: "cargo test", cwd: `${directory}/src-tauri` },
  ]

  return {
    event: async ({ event }) => {
      if (event.type !== "session.idle") return
      if (running) return
      running = true

      try {
        for (const step of steps) {
          const result = await $`sh -c ${step.cmd}`.cwd(step.cwd).quiet().nothrow()
          if (result.exitCode !== 0) {
            const tail = (result.stderr?.toString() || result.stdout?.toString() || "")
              .trim()
              .split("\n")
              .slice(-8)
              .join("\n")
            await client.app.log({
              body: {
                service: "verify-on-idle",
                level: "error",
                message: `${step.label} failed (exit ${result.exitCode})`,
                extra: { cmd: step.cmd, tail },
              },
            })
            await client.tui.showToast({
              body: {
                title: "Verify failed",
                message: `${step.label} 失败，见日志`,
                variant: "error",
                duration: 8000,
              },
            })
            return
          }
        }
      } finally {
        running = false
      }
    },
  }
}
