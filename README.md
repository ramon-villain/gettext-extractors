# Gettext-Extractor.rs 🦀

## Introduction

This is a work-in-progress Gettext message extractor written in Rust, with support for JavaScript, TypeScript, and JSX files. It is inspired by [lukasgeiter/gettext-extractor](https://github.com/lukasgeiter/gettext-extractor).

## Usage

You can use `gettext-extractor` by either providing a configuration file or piping the content directly to the program.

### Using a Configuration File

Provide a configuration file as an argument:

```bash
gettext-extractor -c /path/to/config.json
```

Here's an example `config.json`:

<details>
<summary>Click to see the example config.json</summary>

```json
{
  "functions": {
    "gettext": {
      "text": 0
    },
    "ngettext": {
      "text": 0,
      "plural": 1
    },
    "pgettext": {
      "text": 1,
      "context": 0
    },
    "npgettext": {
      "context": 0,
      "text": 1,
      "plural": 2
    }
  },
  "exclude": [
    "!{node_modules}",
    "!*{test}.{tsx,ts}"
  ],
  "include": [
    "src/**/*.{ts,tsx}"
  ],
  "base": "/path/to/your-project"
}
```

</details>

### Piping Content to the Program

Alternatively, you can pipe the content to the program:

```bash
./gettext-extractor -b /path/to/your-project/ -e "\!{node_modules}/" -e '!*{test,stories}.{tsx,ts}' -i "src/**/*.{ts,tsx}"
```

Feel free to use whichever method suits your needs best for extracting Gettext messages from your codebase.

Please note that this project is still a work in progress, and feedback or contributions are highly appreciated!
