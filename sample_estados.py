import re

with open('/home/rafael/claude/hege-wt/a-distribuicao/assets/data/economia/estados.ron') as f:
    for i, line in enumerate(f):
        if i < 20:
            print(line.strip())
