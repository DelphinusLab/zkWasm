## How to compile the .c files
run `make`

## How to install clang-15
1. Add apt source address:
```
wget -O - https://apt.llvm.org/llvm-snapshot.gpg.key|sudo apt-key add - 
```
2. Check the source added successfully:
run
```
sudo apt-key list
```
and find the hash value below is added:
Fingerprint: 6084 F3CF 814B 57C1 CF12 EFD5 15CF 4D18 AF4F 7421

3. update the apt-key:
```
sudo apt-key update 
```

4. Run the command to install clang-15
```
apt-get install clang-15 lldb-15 lld-15
```
This will install clang, lld and lldb (15 release)

Ref: https://apt.llvm.org/