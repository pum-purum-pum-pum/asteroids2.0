import os;

pref = "impact"
i = 1
for filename in os.listdir("."):
    if filename.startswith(pref):
        os.rename(filename, str(i) + ".png")
        i = i + 1
#         os.rename(filename, filename.replace("0", ""))
#         print(filename)