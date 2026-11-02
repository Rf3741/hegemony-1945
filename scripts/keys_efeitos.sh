#!/usr/bin/env bash
grep -rhoE 'Modificador\([[:space:]]*id:[[:space:]]*"[^"]+"' assets/data/eventos \
  | sed -E 's/.*id:[[:space:]]*"//' \
  | sort | uniq -c | sort -rn
