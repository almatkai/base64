/// <reference types="@raycast/api">

/* 🚧 🚧 🚧
 * This file is auto-generated from the extension's manifest.
 * Do not modify manually. Instead, update the `package.json` file.
 * 🚧 🚧 🚧 */

/* eslint-disable @typescript-eslint/ban-types */

type ExtensionPreferences = {
  /** Default Action - Choose the first action shown for a result. */
  "defaultAction": "copyToClipboard" | "pasteInApp" | "openInBrowser"
}

/** Preferences accessible in all the extension's commands */
declare type Preferences = ExtensionPreferences

declare namespace Preferences {
  /** Preferences accessible in the `index` command */
  export type Index = ExtensionPreferences & {}
  /** Preferences accessible in the `encode` command */
  export type Encode = ExtensionPreferences & {}
  /** Preferences accessible in the `decode` command */
  export type Decode = ExtensionPreferences & {}
}

declare namespace Arguments {
  /** Arguments passed to the `index` command */
  export type Index = {}
  /** Arguments passed to the `encode` command */
  export type Encode = {}
  /** Arguments passed to the `decode` command */
  export type Decode = {}
}

