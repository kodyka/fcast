import sys

def read_chunk(filepath, start_line, num_lines=15):
    with open(filepath, 'r') as f:
        lines = f.readlines()

    start_idx = start_line - 1
    end_idx = start_idx + num_lines
    chunk = lines[start_idx:end_idx]

    print(f"--- {filepath} lines {start_line}-{start_line + len(chunk) - 1} ---")
    for i, line in enumerate(chunk):
        print(f"{start_idx + i + 1:4d}: {line}", end='')

if __name__ == '__main__':
    read_chunk(sys.argv[1], int(sys.argv[2]), int(sys.argv[3]))
