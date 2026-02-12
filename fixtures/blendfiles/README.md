This directory is storage space for .blend files to test on. All .blend files are ignored in this repo.

To download fixture files listed in `blendfiles_map.json`, run the downloader binary from the workspace root:
	- cargo run -p blendoc_blendfiles_downloader -- --root blendfiles
	- Add --dry-run to see planned downloads.
	- Add --folder <name> to restrict to one top-level fixture folder.
	- Add --overwrite to refresh files that already exist.
