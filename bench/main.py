from benchmarks import baseline

BENCHMARKS = [
    baseline,
]


def main():
    for bench in BENCHMARKS:
        print(f"── {bench.NAME} ──")
        bench.run()
        print()


if __name__ == "__main__":
    main()
