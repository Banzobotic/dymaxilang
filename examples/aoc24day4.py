import time

start = time.time()
file = open("input", "r")
input = file.read().strip()

for _ in range(5):
    input += "\n" + input

count = 0
lines = list(input.splitlines())

for line in lines:
    for i in range(len(line) - 3):
        word = line[i:i+4]
        if word == "XMAS" or word == "SAMX":
            count += 1

for i in range(len(lines[0])):
    for j in range(len(lines) - 3):
        word = lines[j][i] + lines[j + 1][i] + lines[j + 2][i] + lines[j + 3][i]
        if word == "XMAS" or word == "SAMX":
            count += 1

for j in range(len(lines[0]) - 3):
    for i in range(len(lines) - 3):
        word = lines[i][j] + lines[i + 1][j + 1] + lines[i + 2][j + 2] + lines[i + 3][j + 3]
        if word == "XMAS" or word == "SAMX":
            count += 1
            
for j in range(3, len(lines[0])):
    for i in range(len(lines) - 3):
        word = lines[i][j] + lines[i + 1][j - 1] + lines[i + 2][j - 2] + lines[i + 3][j - 3]
        if word == "XMAS" or word == "SAMX":
            count += 1
print(count)

count = 0
for i in range(len(lines) - 2):
    for j in range(len(lines[0]) - 2):
        word1 = lines[i][j] + lines[i + 1][j + 1] + lines[i + 2][j + 2]
        word2 = lines[i + 2][j] + lines[i + 1][j + 1] + lines[i][j + 2]
        if (word1 == "MAS" or word1 == "SAM") and (word2 == "MAS" or word2 == "SAM"):
            count += 1
print(count)
print(time.time() - start)
