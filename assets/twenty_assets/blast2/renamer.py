import os;

pref = "blast"

for (i, filename) in enumerate(os.listdir(".")):
    if filename.startswith(pref):
        os.rename(filename, str(1 + i) + ".png")
#         os.rename(filename, filename.replace("0", ""))
#         print(filename)