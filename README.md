# Unblend

A small command-line program that explodes Blender files (`.blend`) into their various parts and writes them out as an archive.

## Features

- Accepts a `blend`-file, either as file-path or thru STDIN (using `-` as file).

- Parses the blocks the file is made out of.
  - The block-`code` is used as directory.
  - The block-`address` is used as file-name.
  - Block-data is written to `<CODE>/<ADDR>.bin`
  - Respective metadata to `<CODE>/<ADDR>.txt`

- Almost fully decodes the `DNA1` block.
  - See the resulting `DNA1.tsv` and `DNA1/*.txt` files.

- Outputs an archive in either `*.zip` or `*.tar` format.
  - Output can go to STDOUT via `-` (but only as `*.tar`).

- Excluding data from the archive being written, via `-x <GLOB>`.

## References

- <https://www.atmind.nl/blender/blender-sdna-256.html>
- <https://fossies.org/linux/blender/doc/blender_file_format/mystery_of_the_blend.html>
- <https://harlepengren.com/blender-dna-unraveling-the-internal-structure/>
