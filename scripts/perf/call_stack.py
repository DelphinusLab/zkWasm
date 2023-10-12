import json
import copy
import sys

def analyze_etable(etable):
    dict = {}
    frame = []

    for entry in etable:
        if len(frame) == 0:
            frame.append(entry["inst"]["function_name"])

        if type(entry["inst"]["opcode"]) == str:
            opcode = entry["inst"]["opcode"]
        else:
            if len(entry["inst"]["opcode"].items()) != 1:
                print("Panic")
                exit(1)

            for key, value in entry["inst"]["opcode"].items():
                opcode = key
                break

        if tuple(frame) not in dict:
                dict[tuple(frame)] = 1
        else:
                dict[tuple(frame)] = dict[tuple(frame)] + 1

        if opcode == "Call":
            frame.append(entry["step_info"]["Call"]["function_name"])
        elif opcode == "CallIndirect":
            frame.append(entry["step_info"]["CallIndirect"]["function_name"])
        elif opcode == "Return":
            frame.pop()
    
    return dict

def generate_log(dict):
    for key, value in dict.items():
        print(*key, sep=";", end="")
        print(" ", end="")
        print(value)

def main():
    if len(sys.argv) != 2:
        print("Usage: python call_stack.py <Path of etable.json>");
        exit(1)

    etable_path = sys.argv[1]
    etable = open(etable_path)
    etable = json.load(etable)

    dict = analyze_etable(etable)
    generate_log(dict)

main()
