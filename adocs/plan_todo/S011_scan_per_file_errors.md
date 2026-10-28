id:         S011
goal:       source scan reports unreadable entries per file instead of failing the whole source
accepts:    an unreadable root or DCIM is still an `Err`; an unreadable file or sub-folder under DCIM goes into the scan as a per-entry failure with its path and error, and every other file is still listed; tests simulate it (e.g. a sub-folder with no read permission, unix); existing source_scan tests stay green unchanged; INV-1 holds
touches:    photosync-core source scan
excludes:   report formatting (S006); duplicate fallback (S010)
closes:
paused_by:
author:
done:
