import assert from 'node:assert/strict'
import test from 'node:test'
import {
  buildMicrosoftStoreUri,
  buildMicrosoftStoreSearchUri,
  getDetailsEntryForSelectedPlatform,
  getLaunchChoices,
  getLaunchActionState,
  getPreferredLaunchEntryId,
  getVisibleSelectedEntry,
  isSteamInstallUri,
  resolveMicrosoftStoreTarget,
} from './libraryLaunch.js'

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
    sources: [{ platformId: 'xbox', externalId: '9NBLGGH4R315' }],
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
  assert.equal(xboxStoreState.primaryLaunchAction?.target, 'ms-windows-store://pdp/?productid=9NBLGGH4R315')
})

test('getLaunchActionState keeps Xbox installed entries executable', () => {
  const xboxInstalledState = getLaunchActionState(xboxInstalledEntry)

  assert.equal(xboxInstalledState.canLaunch, true)
  assert.equal(xboxInstalledState.primaryLaunchAction?.kind, 'executable')
  assert.equal(xboxInstalledState.primaryLaunchAction?.target, 'C:/Games/Xbox/Test.exe')
})

test('getLaunchActionState creates a Steam install action for not installed Steam entries with AppID source', () => {
  const steamInstallState = getLaunchActionState({
    id: 'entry-steam-remote',
    primaryPlatformId: 'steam',
    installStatus: 'not_installed',
    game: {
      sources: [{ platformId: 'steam', externalId: '413150' }],
      launchActions: [
        {
          isPrimary: true,
          kind: 'uri',
          target: 'steam://rungameid/413150',
          label: 'Steam',
        },
      ],
    },
  })

  assert.equal(steamInstallState.canLaunch, true)
  assert.equal(steamInstallState.primaryLaunchAction?.label, 'Instalar')
  assert.equal(steamInstallState.primaryLaunchAction?.target, 'steam://install/413150')
})

test('getLaunchActionState avoids Steam install promise when AppID is unavailable', () => {
  const steamInstallState = getLaunchActionState({
    id: 'entry-steam-unknown',
    primaryPlatformId: 'steam',
    installStatus: 'not_installed',
    game: {
      sources: [{ platformId: 'steam', externalId: '' }],
      launchActions: [],
    },
  })

  assert.equal(steamInstallState.canLaunch, false)
  assert.equal(steamInstallState.primaryLaunchAction, null)
  assert.match(steamInstallState.hint, /acao de lancamento/i)
})

test('isSteamInstallUri identifies Steam install links only', () => {
  assert.equal(isSteamInstallUri('steam://install/413150'), true)
  assert.equal(isSteamInstallUri('steam://rungameid/413150'), false)
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

test('resolveMicrosoftStoreTarget ignores non store xbox external ids when building pdp links', () => {
  const target = resolveMicrosoftStoreTarget({
    primaryPlatformId: 'xbox',
    game: {
      sources: [{ platformId: 'xbox', externalId: '124321' }],
      launchActions: [],
    },
  })

  assert.equal(target, '')
})

test('buildMicrosoftStoreUri normalizes product ids', () => {
  assert.equal(buildMicrosoftStoreUri('  abc123  '), 'ms-windows-store://pdp/?productid=abc123')
})

test('buildMicrosoftStoreSearchUri trims generic launcher suffixes from the query', () => {
  assert.equal(
    buildMicrosoftStoreSearchUri('Minecraft Launcher'),
    'ms-windows-store://search/?query=Minecraft',
  )
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
              platformId: 'steam',
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
          sources: [{ platformId: 'xbox', externalId: '9NBLGGH4R315' }],
          launchActions: [],
        },
      },
    ],
  }

  const launchChoices = getLaunchChoices(groupedEntry)

  assert.equal(launchChoices.length, 2)
  assert.equal(launchChoices[0]?.platformLabel, 'Steam')
  assert.equal(launchChoices[1]?.platformLabel, 'Xbox')
  assert.equal(launchChoices[1]?.launchAction?.target, 'ms-windows-store://pdp/?productid=9NBLGGH4R315')
})

test('getLaunchChoices includes Epic manifest launch actions when present', () => {
  const groupedEntry = {
    memberEntries: [
      {
        id: 'steam-entry',
        primaryPlatformId: 'steam',
        game: {
          launchActions: [
            {
              isPrimary: true,
              platformId: 'steam',
              kind: 'uri',
              target: 'steam://rungameid/10',
              label: 'Jogar na Steam',
            },
          ],
        },
      },
      {
        id: 'epic-entry',
        primaryPlatformId: 'epic',
        installStatus: 'installed',
        game: {
          launchActions: [
            {
              isPrimary: true,
              platformId: 'epic',
              kind: 'uri',
              target: 'com.epicgames.launcher://apps/ns%3Aitem%3Aartifact?action=launch&silent=true',
              label: 'Epic Games',
            },
          ],
        },
      },
    ],
  }

  const launchChoices = getLaunchChoices(groupedEntry, 'epic')

  assert.equal(launchChoices.length, 2)
  assert.equal(launchChoices[0]?.platformLabel, 'Epic Games')
  assert.equal(launchChoices[0]?.entryId, 'epic-entry')
  assert.equal(
    launchChoices[0]?.launchAction?.target,
    'com.epicgames.launcher://apps/ns%3Aitem%3Aartifact?action=launch&silent=true',
  )
})

test('getLaunchActionState does not create a fake Epic action when no launch action exists', () => {
  const epicState = getLaunchActionState({
    id: 'entry-epic-no-action',
    primaryPlatformId: 'epic',
    installStatus: 'installed',
    game: {
      sources: [{ platformId: 'epic', externalId: 'ns:item:artifact' }],
      launchActions: [],
    },
  })

  assert.equal(epicState.canLaunch, false)
  assert.equal(epicState.primaryLaunchAction, null)
  assert.match(epicState.hint, /acao de lancamento/i)
})

test('getLaunchActionState falls back to store search when no product id is available', () => {
  const xboxSearchState = getLaunchActionState({
    id: 'entry-search',
    primaryPlatformId: 'xbox',
    installStatus: 'not_installed',
    game: {
      title: 'Minecraft Launcher',
      sources: [{ platformId: 'xbox', externalId: '124321' }],
      launchActions: [],
    },
  })

  assert.equal(xboxSearchState.canLaunch, true)
  assert.equal(
    xboxSearchState.primaryLaunchAction?.target,
    'ms-windows-store://search/?query=Minecraft',
  )
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
              platformId: 'steam',
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
          sources: [{ platformId: 'xbox', externalId: '9NBLGGH4R315' }],
          launchActions: [],
        },
      },
    ],
  }

  const launchChoices = getLaunchChoices(groupedEntry, 'xbox')

  assert.equal(launchChoices[0]?.platformLabel, 'Xbox')
  assert.equal(launchChoices[1]?.platformLabel, 'Steam')
})

test('getLaunchChoices deduplicates repeated entries from the same platform', () => {
  const groupedEntry = {
    memberEntries: [
      {
        id: 'steam-entry',
        primaryPlatformId: 'steam',
        game: {
          launchActions: [
            {
              isPrimary: true,
              platformId: 'steam',
              kind: 'uri',
              target: 'steam://rungameid/10',
              label: 'Jogar na Steam',
            },
          ],
        },
      },
      {
        id: 'xbox-folder-entry',
        primaryPlatformId: 'xbox',
        installStatus: 'installed',
        game: {
          launchActions: [
            {
              isPrimary: true,
              platformId: 'xbox',
              kind: 'executable',
              target: 'E:\\XboxGames\\Clair Obscur- Expedition 33\\Content\\SandFall.exe',
              label: 'Jogar no Xbox',
            },
          ],
        },
      },
      {
        id: 'xbox-aumid-entry',
        primaryPlatformId: 'xbox',
        installStatus: 'installed',
        game: {
          launchActions: [
            {
              isPrimary: true,
              platformId: 'xbox',
              kind: 'executable',
              target: 'C:\\Windows\\explorer.exe',
              label: 'Jogar no Xbox',
            },
          ],
        },
      },
    ],
  }

  const launchChoices = getLaunchChoices(groupedEntry, 'xbox')

  assert.equal(launchChoices.length, 2)
  assert.deepEqual(launchChoices.map((choice) => choice.platformId), ['xbox', 'steam'])
  assert.equal(launchChoices[0]?.entryId, 'xbox-folder-entry')
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
          sources: [{ platformId: 'xbox', externalId: '9NBLGGH4R315' }],
          launchActions: [],
        },
      },
    ],
  }

  assert.equal(getPreferredLaunchEntryId(groupedEntry), 'steam-entry')
})

test('getPreferredLaunchEntryId follows the selected platform preference', () => {
  const groupedEntry = {
    memberEntries: [
      {
        id: 'steam-entry',
        primaryPlatformId: 'steam',
        game: {
          launchActions: [
            {
              isPrimary: true,
              platformId: 'steam',
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
        installStatus: 'installed',
        game: {
          launchActions: [
            {
              isPrimary: true,
              platformId: 'xbox',
              kind: 'executable',
              target: 'C:\\Windows\\explorer.exe',
              label: 'Jogar no Xbox',
            },
          ],
        },
      },
    ],
  }

  assert.equal(getPreferredLaunchEntryId(groupedEntry, 'xbox'), 'xbox-entry')
  assert.equal(getPreferredLaunchEntryId(groupedEntry, 'steam'), 'steam-entry')
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
          sources: [{ platformId: 'xbox', externalId: '9NBLGGH4R315' }],
          launchActions: [],
        },
      },
    ],
  }

  const preferredXboxState = getLaunchActionState(groupedEntry, 'xbox')
  const preferredSteamState = getLaunchActionState(groupedEntry, 'steam')

  assert.equal(preferredSteamState.primaryLaunchAction?.platformId, 'steam')
  assert.equal(preferredSteamState.primaryLaunchAction?.label, 'Jogar na Steam')
  assert.equal(preferredSteamState.primaryLaunchAction?.target, 'steam://rungameid/10')

  assert.equal(preferredXboxState.primaryLaunchAction?.platformId, 'xbox')
  assert.equal(preferredXboxState.primaryLaunchAction?.label, 'Abrir Microsoft Store')
  assert.equal(
    preferredXboxState.primaryLaunchAction?.target,
    'ms-windows-store://pdp/?productid=9NBLGGH4R315',
  )
})

test('getDetailsEntryForSelectedPlatform returns the member entry matching the selected launcher', () => {
  const groupedEntry = {
    installStatus: 'installed',
    game: {
      playtime: { totalMinutes: 999 },
    },
    memberEntries: [
      {
        id: 'steam-entry',
        primaryPlatformId: 'steam',
        installStatus: 'installed',
        lastPlayedLabel: 'Ontem',
        game: {
          playtime: { totalMinutes: 300 },
          launchActions: [
            {
              isPrimary: true,
              platformId: 'steam',
              kind: 'uri',
              target: 'steam://rungameid/10',
              label: 'Jogar na Steam',
              workingDirectory: 'D:\\SteamLibrary\\steamapps\\common\\Hollow Knight',
            },
          ],
        },
      },
      {
        id: 'xbox-entry',
        primaryPlatformId: 'xbox',
        installStatus: 'not_installed',
        lastPlayedLabel: 'Nunca',
        game: {
          title: 'Hollow Knight',
          sources: [{ platformId: 'xbox', externalId: '9NBLGGH4R315' }],
          playtime: { totalMinutes: 60 },
          launchActions: [],
        },
      },
    ],
  }

  const steamDetailsEntry = getDetailsEntryForSelectedPlatform(groupedEntry, 'steam')
  const xboxDetailsEntry = getDetailsEntryForSelectedPlatform(groupedEntry, 'xbox')

  assert.equal(steamDetailsEntry.id, 'steam-entry')
  assert.equal(steamDetailsEntry.game.playtime.totalMinutes, 300)
  assert.equal(steamDetailsEntry.lastPlayedLabel, 'Ontem')

  assert.equal(xboxDetailsEntry.id, 'xbox-entry')
  assert.equal(xboxDetailsEntry.game.playtime.totalMinutes, 60)
  assert.equal(xboxDetailsEntry.installStatus, 'not_installed')
})
