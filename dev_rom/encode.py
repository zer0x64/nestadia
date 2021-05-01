import itertools

key = b"\xa1\x9b\xd8\xdd\x29\xf3\xa7\x77\xd7\x61\x9b\x4b\x72\x90\x45\xc8\x4a\xea\x81\x93\xd5\xaf\x9a\x75\x2b\xec\x6c\xf7\xb7\x47\x50\x4d"

with open("hello_world.nes", 'rb') as f:
    rom = f.read()

enc = bytes([r ^ k for (r, k) in zip(rom, itertools.cycle(key))])

with open("devrom.bin", "wb") as f:
    f.write(enc)
