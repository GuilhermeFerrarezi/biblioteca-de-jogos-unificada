import assert from 'node:assert/strict'
import test from 'node:test'
import {
  buildMicrosoftStoreUri,
  getLaunchChoices,
  getLaunchActionState,
  getPreferredLaunchEntryId,
  getVisibleSelectedEntry,
  resolveMicrosoftStoreTarget,
} from './libraryPageStateHelpers.js'

const executableEntry = {
  id: 'entry-1',
  game: {
    launchActions: [
      {
        isPrimary: true,
        kind: 'executable',
        target: 'C:/Games/Test/Game.exe',
      },
    ],
  },
}

const manualEntry = {
  id: 'entry-2',
  game: {
    launchActions: [
      {
        isPrimary: true,
        kind: 'manual',
        target: '',
      },
    ],
  },
}

const xboxInstalledEntry = {
  id: 'entry-3',
  primaryPlatformId: 'xbox',
  installStatus: 'installed',
  game: {
    launchActions: [
      {
        isPrimary: true,
        kind: 'executable',
        target: 'C:/Games/Xbox/Test.exe',
      },
    ],
  },
}

const xboxStoreEntry = {
  id: 'entry-4',
  primaryPlatformId: 'xbox',
  installStatus: 'not_installed',
  game: {
    sources: [{ platformId: 'xbox', externalId: '1234567890' }],
    launchActions: [],
  },
}

test('getVisibleSelectedEntry keeps the selected entry when it is visible', () => {
  const selectedEntry = getVisibleSelectedEntry([executableEntry, manualEntry], 'entry-2')

  assert.equal(selectedEntry?.id, 'entry-2')
})

test('getVisibleSelectedEntry falls back to the first filtered entry when selection is hidden', () => {
  const selectedEntry = getVisibleSelectedEntry([executableEntry, manualEntry], 'missing-entry')

  assert.equal(selectedEntry?.id, 'entry-1')
})

test('getVisibleSelectedEntry returns null when there are no filtered entries', () => {
  assert.equal(getVisibleSelectedEntry([], 'entry-1'), null)
})

test('getLaunchActionState enables launch only for executable or uri actions', () => {
  const executableState = getLaunchActionState(executableEntry)
  const manualState = getLaunchActionState(manualEntry)

  assert.equal(executableState.canLaunch, true)
  assert.equal(executableState.hint, '')
  assert.equal(manualState.canLaunch, false)
  assert.match(manualState.hint, /acao executavel/i)
})

test('getLaunchActionState creates a Microsoft Store action for Xbox entries that are not installed', () => {
  const xboxStoreState = getLaunchActionState(xboxStoreEntry)

  assert.equal(xboxStoreState.canLaunch, true)
  assert.equal(xboxStoreState.primaryLaunchAction?.label, 'Abrir Microsoft Store')
  assert.equal(xboxStoreState.primaryLaunchAction?.target, 'ms-windows-store://pdp/?productid=1234567890')
})

test('getLaunchActionState keeps Xbox installed entries executable', () => {
  const xboxInstalledState = getLaunchActionState(xboxInstalledEntry)

  assert.equal(xboxInstalledState.canLaunch, true)
  assert.equal(xboxInstalledState.primaryLaunchAction?.kind, 'executable')
  assert.equal(xboxInstalledState.primaryLaunchAction?.target, 'C:/Games/Xbox/Test.exe')
})

test('resolveMicrosoftStoreTarget keeps direct Microsoft Store links intact', () => {
  const directStoreEntry = {
    primaryPlatformId: 'xbox',
    game: {
      storeUri: 'ms-windows-store://pdp/?productid=abc123',
      sources: [],
      launchActions: [],
    },
  }

  assert.equal(resolveMicrosoftStoreTarget(directStoreEntry), 'ms-windows-store://pdp/?productid=abc123')
})

test('buildMicrosoftStoreUri normalizes product ids', () => {
  assert.equal(buildMicrosoftStoreUri('  abc123  '), 'ms-windows-store://pdp/?productid=abc123')
})

test('getLaunchChoices returns one option per underlying platform entry', () => {
  const groupedEntry = {
    memberEntries: [
      {
        id: 'steam-entry',
        primaryPlatformId: 'steam',
        game: {
          launchActions: [
            {
              isPrimary: true,
              kind: 'uri',
              target: 'steam://rungameid/10',
              label: 'Jogar na Steam',
            },
          ],
        },
      },
      {
        id: 'xbox-entry',
        primaryPlatformId: 'xbox',
        installStatus: 'not_installed',
        game: {
          sources: [{ platformId: 'xbox', externalId: '1234567890' }],
          launchActions: [],
        },
      },
    ],
  }

  const launchChoices = getLaunchChoices(groupedEntry)

  assert.equal(launchChoices.length, 2)
  assert.equal(launchChoices[0]?.platformLabel, 'Steam')
  assert.equal(launchChoices[1]?.platformLabel, 'Xbox')
  assert.equal(launchChoices[1]?.launchAction?.target, 'ms-windows-store://pdp/?productid=1234567890')
})

test('getLaunchChoices prioritizes the preferred platform first', () => {
  const groupedEntry = {
    memberEntries: [
      {
        id: 'steam-entry',
        primaryPlatformId: 'steam',
        game: {
          launchActions: [
            {
              isPrimary: true,
              kind: 'uri',
              target: 'steam://rungameid/10',
              label: 'Jogar na Steam',
            },
          ],
        },
      },
      {
        id: 'xbox-entry',
        primaryPlatformId: 'xbox',
        installStatus: 'not_installed',
        game: {
          sources: [{ platformId: 'xbox', externalId: '1234567890' }],
          launchActions: [],
        },
      },
    ],
  }

  const launchChoices = getLaunchChoices(groupedEntry, 'xbox')

  assert.equal(launchChoices[0]?.platformLabel, 'Xbox')
  assert.equal(launchChoices[1]?.platformLabel, 'Steam')
})

test('getPreferredLaunchEntryId picks the first launchable option from grouped entries', () => {
  const groupedEntry = {
    memberEntries: [
      {
        id: 'steam-entry',
        primaryPlatformId: 'steam',
        game: {
          launchActions: [
            {
              isPrimary: true,
              kind: 'uri',
              target: 'steam://rungameid/10',
              label: 'Jogar na Steam',
            },
          ],
        },
      },
      {
        id: 'xbox-entry',
        primaryPlatformId: 'xbox',
        installStatus: 'not_installed',
        game: {
          sources: [{ platformId: 'xbox', externalId: '1234567890' }],
          launchActions: [],
        },
      },
    ],
  }

  assert.equal(getPreferredLaunchEntryId(groupedEntry), 'steam-entry')
})

test('getLaunchActionState honors the preferred platform for grouped entries', () => {
  const groupedEntry = {
    memberEntries: [
      {
        id: 'steam-entry',
        primaryPlatformId: 'steam',
        game: {
          launchActions: [
            {
              isPrimary: true,
              kind: 'uri',
              target: 'steam://rungameid/10',
              label: 'Jogar na Steam',
            },
          ],
        },
      },
      {
        id: 'xbox-entry',
        primaryPlatformId: 'xbox',
        installStatus: 'not_installed',
        game: {
          sources: [{ platformId: 'xbox', externalId: '1234567890' }],
          launchActions: [],
        },
      },
    ],
  }

  const preferredXboxState = getLaunchActionState(groupedEntry, 'xbox')

  assert.equal(preferredXboxState.primaryLaunchAction?.platformId, 'xbox')
  assert.equal(preferredXboxState.primaryLaunchAction?.label, 'Abrir Microsoft Store')
})
