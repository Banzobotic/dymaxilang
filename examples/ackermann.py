import time, sys

sys.setrecursionlimit(10000)

def ackermann(m, n):
    if m == 0:
        return n + 1
    else:
        if n == 0:
            return ackermann(m - 1, 1)
        else:
            return ackermann(m - 1, ackermann(m, n - 1))

start = time.time();
print(ackermann(3, 10))
print(time.time() - start)
