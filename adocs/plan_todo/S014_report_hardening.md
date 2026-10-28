id:         S014
goal:       report hardening from the S006 fast check: no false "skipped", visible not-flushed, stricter Report::new
accepts:    a duplicate whose original failed or was cancelled is never reported as skipped: it takes the original's bucket with a reason naming the original, tested for a failed (read error, name collision) and a cancelled original, on two cards; `FailReason::NotFlushed` stays in Failed, and `Tally` exposes it separately so a front-end can show "in place, flush not confirmed"; INV-6 wording in specs says both; `Report::new` refuses a source root given twice (any spelling) and a plan or outcome whose file size or mtime differs from the scan's; `Report` fields are private, with read accessors, so the checks cannot be bypassed; each item gets a failing test first; scripts/test_exfat.sh's second report entry gets its `# report` comment
touches:    photosync-core report, adocs/specs.md, scripts/test_exfat.sh
excludes:   the duplicate fallback copy itself (S010); text formatting
closes:
paused_by:
author:
done:
