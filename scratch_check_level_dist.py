import os
import subprocess
import re
from collections import Counter

os.chdir("/home/rafael/claude/hege-wt/a-distribuicao")

proc = subprocess.run(["git", "show", "jogavel:assets/data/economia/provincias.ron"], capture_output=True, text=True)
j_text = proc.stdout

level_counts = Counter()
for line in j_text.split("\n"):
    if 'tipo_predio_id:' in line:
        for m in re.finditer(r'nivel:(\d+)', line):
            lvl = int(m.group(1))
            if lvl <= 5:
                level_counts[lvl] += 1
            else:
                level_counts['>5'] += 1

print("Jogavel level distribution:", level_counts)
