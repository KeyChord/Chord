import { defineConfig } from 'reactive-vscode'

export const config = defineConfig<{
  message: string
}>('chordsapp-command-runner')
