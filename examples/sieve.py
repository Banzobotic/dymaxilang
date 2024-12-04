import time

start = time.time()

sieve = [True] * 4000001

for i in range(2, 2001):
    if sieve[i]:
        j = i * i
        while j <= 4000000:
            sieve[j] = False;
            j += i

count = 0
for i in range(2, 4000001):
    if sieve[i]:
        count += 1

print(count)
print(time.time() - start)
