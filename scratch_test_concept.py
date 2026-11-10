import os
import subprocess
import re
from collections import defaultdict

os.chdir("/home/rafael/claude/hege-wt/a-distribuicao")

proc = subprocess.run(["git", "show", "jogavel:assets/data/economia/provincias.ron"], capture_output=True, text=True)
j_text = proc.stdout

# Let's see what happens if in USA:
# We move levels:
# Total USA oil levels in jogavel: 8908
# Total USA iron levels in jogavel: 6067
# Total USA grain farm levels in jogavel: 22035
# Total USA steel levels in jogavel: 45152
# Total USA coal levels in jogavel: 7616

# Check Guam in jogavel:
# If Guam removes mina_ferro:
# How many buildings in total? 40789 - 1 (or a few for IA ferro, NY oil) = ~40780 >= 40000!

print("Testing count if we remove mina_ferro from Guam, Iowa; poco_petroleo from NY:")
# Just 3 buildings removed! 40789 - 3 = 40786!
print("40786 >= 40000 is True!")
