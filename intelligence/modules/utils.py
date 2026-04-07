from pathlib import Path


def find_absolute_path():
    script_path = Path(__file__).resolve()
    absolute__path = script_path.__str__().split("\\")
    absolute__path = "\\".join(absolute__path[:len(absolute__path) - 1])
    return absolute__path
