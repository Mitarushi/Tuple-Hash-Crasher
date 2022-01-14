hash_count_dict = {}
input_count_set = set()

count = 0
while True:
    try:
        line = input().strip()
        if line == '':
            break

        count += 1
        input_count_set.add(line)

        line_hash = hash(tuple(map(int, line.split())))
        hash_count_dict[line_hash] = hash_count_dict.get(line_hash, 0) + 1
    except:
        break

print("Hash Count:", hash_count_dict)
print("Number of Unique Inputs:", len(input_count_set))
print("Number of Inputs:", count)

if len(input_count_set) == count:
    print("All inputs are unique")
else:
    print("ERROR: Some inputs are not unique")

if len(hash_count_dict) == 1:
    print("All hash values are the same")
else:
    print("ERROR: Some hash values are not the same")
