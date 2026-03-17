import { defineExtension, useCommand, useIsDarkTheme, watchEffect } from 'reactive-vscode'
import { window } from 'vscode'
import { config } from './config'
import { logger } from './utils'

export = defineExtension(() => {
  logger.info('Extension Activated')

  useCommand('chordsapp-command-runner.helloWorld', () => {
    window.showInformationMessage(config.message)
  })

  const isDark = useIsDarkTheme()
  watchEffect(() => {
    logger.info('Is Dark Theme:', isDark.value)
  })
})
