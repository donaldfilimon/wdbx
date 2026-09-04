# WDBX Specimen Studio

A working local specimen studio with the complete 26-chapter WDBX architecture specification built into its reference reader. The white and teal interface includes a selectable topology, node inspector, cycle trace, conversation, supporting memory, learning proposals, and an activity journal.

## Use the studio

1. Run **What is 2 + 2?** to inspect an actual arithmetic vote and response.
2. Open **Node Library → Add pattern**. Enter an original pattern and its response, then prompt that pattern in Studio.
3. Turn on **Contributors** to see which node supplied each output segment. **Right/Wrong** feedback performs an independent coin flip per contributor and changes strength when selected.
4. Add supporting records in **Memory** and optionally link them to a Pattern ID. Pin conversation records to retain them beyond rolling history.
5. **Save specimen** downloads the complete workspace as JSON. **Load** validates a file and previews its contents before replacement. Device storage also saves automatically through IndexedDB. A failed recovery pauses autosave to preserve existing data.
6. Read or search **Specification**, and download the full Markdown document from the reader.

`/prompt`, `/right`, `/wrong`, `/wrng`, `/contributorfractal`, `/saveSpecimen`, `/loadSpecimen`, `/addPattern`, and `/learn` are recognized by the studio composer. Add/load/learn commands open the corresponding visible editor or file chooser. `/learn` accepts a pending proposal ID or exact proposal text, or defaults to the first pending proposal.

## Local development

```sh
bun install
bun dev
bun test
bunx tsc --noEmit
bun build
```

The project uses React, TypeScript, Vinext/Vite, and the Sites Worker packaging workflow. It has no model API key requirement. Specimen content is processed in the browser and stored on the current device and origin. Use a JSON download to move work between the local preview and the hosted studio.

## Architecture and runtime boundary

The full specification is in [the downloadable Markdown](public/WDBX-Specimen-Architecture-Specification.md). It is preserved as the design authority. [RUNTIME-PROFILE.md](RUNTIME-PROFILE.md) documents the concrete browser implementation, limits, and extensions that remain outside this studio.
