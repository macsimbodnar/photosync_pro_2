id:         S002
goal:       core scans a source's DCIM tree and sorts files into RAW/JPEG/HEIF/VIDEO/ignored
accepts:    given a temp DCIM tree, the scan returns every file with its type (case-insensitive extensions per specs Layout), size, and modification time; unknown types listed as ignored; a test proves the scan leaves the source manifest unchanged (INV-1)
touches:    photosync-core
excludes:   copying; volume discovery
closes:
paused_by:
author:
done:
