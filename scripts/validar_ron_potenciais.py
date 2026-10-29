import re

with open("/home/rafael/claude/guerra-fria-jogar/assets/data/economia/potenciais.ron", "r", encoding="utf-8") as f:
    text = f.read()

# Check parentheses and brackets matching
stack = []
line_num = 1
col_num = 1
for i, char in enumerate(text):
    if char == '\n':
        line_num += 1
        col_num = 1
    else:
        col_num += 1

    if char in '([{':
        stack.append((char, line_num, col_num))
    elif char in ')]}':
        if not stack:
            print(f"Error: unmatched {char} at {line_num}:{col_num}")
            exit(1)
        top, tl, tc = stack.pop()
        matching = {'(': ')', '[': ']', '{': '}'}
        if matching[top] != char:
            print(f"Mismatch: opened {top} at {tl}:{tc} but got {char} at {line_num}:{col_num}")
            exit(1)

if stack:
    print(f"Unclosed delimiters: {len(stack)}")
    for s, l, c in stack[:5]:
        print(f"  Unclosed {s} from {l}:{c}")
    exit(1)

print("Estrutura de delimitadores (parenteses, chaves, colchetes) está 100% perfeita!")
