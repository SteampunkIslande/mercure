import argparse
import time
import random
import sys
import re


#!/usr/bin/python
def main():
    parser = argparse.ArgumentParser(
        description="Copy template file with random delays between lines"
    )
    parser.add_argument("template_file", help="Path to the template file")
    parser.add_argument("output_file", help="Path to the output file")
    parser.add_argument(
        "--min-delay",
        type=float,
        default=0.1,
        help="Minimum delay between lines (seconds)",
    )
    parser.add_argument(
        "--max-delay",
        type=float,
        default=1.0,
        help="Maximum delay between lines (seconds)",
    )
    args = parser.parse_args()

    try:
        with open(args.template_file, "r") as template, open(
            args.output_file, "w"
        ) as output:
            regex = re.compile(r"(\d+)\s+of\s+(\d+)\s+steps\s+\((1?\d{1,2})%\)\s+done")
            found = False
            for line in template:
                output.write(line)
                output.flush()
                if not found:
                    if regex.search(line):
                        found = True
                    delay = 0
                else:
                    delay = random.uniform(args.min_delay, args.max_delay)
                time.sleep(delay)
    except FileNotFoundError:
        print(f"Error: Template file '{args.template_file}' not found", file=sys.stderr)
        sys.exit(1)
    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
