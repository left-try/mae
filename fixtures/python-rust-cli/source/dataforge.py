#!/usr/bin/env python3
"""
dataforge - Production Data Manipulation & Analytics CLI Utility

Commands:
  stats      Calculate column summaries, numeric aggregates, and null counts
  filter     Filter and sort records based on predicate conditions
  transform  Apply scaling, case conversion, or cryptographic hashing
  inspect    Inspect dataset schema, column types, and record counts

Exit Codes:
  0: Success
  1: Argument validation error
  2: I/O or File Not Found error
  3: Malformed data or parse error
"""

import argparse
import asyncio
from concurrent.futures import ThreadPoolExecutor, ProcessPoolExecutor
import csv
import hashlib
import json
import multiprocessing as mp
import os
import sys
import time
from typing import Any, Dict, List, Optional, Tuple

VERSION = "1.4.0"

# Explicit Exit Code Constants
EXIT_SUCCESS = 0
EXIT_ARG_ERROR = 1
EXIT_IO_ERROR = 2
EXIT_DATA_ERROR = 3


def log(msg: str, verbose: bool = False, quiet: bool = False) -> None:
    if not quiet and (verbose or not verbose):
        sys.stderr.write(f"[dataforge] {msg}\n")


def log_debug(msg: str, verbose: bool = False) -> None:
    if verbose:
        sys.stderr.write(f"[dataforge:debug] {msg}\n")


def read_dataset(input_source: Optional[str], verbose: bool = False) -> Tuple[List[Dict[str, Any]], List[str]]:
    raw_text = ""
    try:
        if input_source is None or input_source == "-":
            log_debug("Reading data from standard input (stdin)...", verbose)
            raw_text = sys.stdin.read()
        else:
            if not os.path.exists(input_source):
                sys.stderr.write(f"Error: input file not found: '{input_source}'\n")
                sys.exit(EXIT_IO_ERROR)
            log_debug(f"Reading data from '{input_source}'...", verbose)
            with open(input_source, "r", encoding="utf-8") as f:
                raw_text = f.read()
    except OSError as e:
        sys.stderr.write(f"I/O error reading input: {e}\n")
        sys.exit(EXIT_IO_ERROR)

    raw_text = raw_text.strip()
    if not raw_text:
        return [], []

    # Attempt JSON parse first
    if raw_text.startswith("[") or raw_text.startswith("{"):
        try:
            parsed = json.loads(raw_text)
            if isinstance(parsed, dict):
                records = [parsed]
            elif isinstance(parsed, list):
                records = [r for r in parsed if isinstance(r, dict)]
            else:
                sys.stderr.write("Error: JSON input must be an array of objects or an object\n")
                sys.exit(EXIT_DATA_ERROR)

            headers = []
            for r in records:
                for k in r.keys():
                    if k not in headers:
                        headers.append(k)
            return records, headers
        except json.JSONDecodeError as e:
            sys.stderr.write(f"Data format error: invalid JSON: {e}\n")
            sys.exit(EXIT_DATA_ERROR)

    # Attempt CSV parse
    try:
        lines = raw_text.splitlines()
        reader = csv.DictReader(lines)
        headers = reader.fieldnames or []
        records = []
        for row in reader:
            parsed_row: Dict[str, Any] = {}
            for k, v in row.items():
                if v is None:
                    parsed_row[k] = None
                    continue
                v_str = str(v).strip()
                # Type inference for numeric values
                try:
                    if "." in v_str:
                        parsed_row[k] = float(v_str)
                    else:
                        parsed_row[k] = int(v_str)
                except ValueError:
                    if v_str.lower() in ("true", "false"):
                        parsed_row[k] = v_str.lower() == "true"
                    else:
                        parsed_row[k] = v_str
            records.append(parsed_row)
        return records, list(headers)
    except Exception as e:
        sys.stderr.write(f"Data format error: failed parsing CSV: {e}\n")
        sys.exit(EXIT_DATA_ERROR)


def write_output(data: Any, output_path: Optional[str], fmt: str = "json", round_digits: Optional[int] = None) -> None:
    output_str = ""
    if fmt == "json":
        if round_digits is not None and isinstance(data, (dict, list)):
            def _round_val(v: Any) -> Any:
                if isinstance(v, float):
                    return round(v, round_digits)
                if isinstance(v, dict):
                    return {k: _round_val(val) for k, val in v.items()}
                if isinstance(v, list):
                    return [_round_val(val) for val in v]
                return v
            data = _round_val(data)
        output_str = json.dumps(data, indent=2, sort_keys=True)
    elif fmt == "csv" and isinstance(data, list) and len(data) > 0 and isinstance(data[0], dict):
        headers = list(data[0].keys())
        lines = [",".join(headers)]
        for row in data:
            row_vals = []
            for h in headers:
                val = row.get(h, "")
                if round_digits is not None and isinstance(val, float):
                    val = round(val, round_digits)
                row_vals.append(str(val) if val is not None else "")
            lines.append(",".join(row_vals))
        output_str = "\n".join(lines)
    else:
        # Plain text fallback
        output_str = str(data)

    if output_path:
        try:
            with open(output_path, "w", encoding="utf-8") as f:
                f.write(output_str + "\n")
        except OSError as e:
            sys.stderr.write(f"I/O error writing to '{output_path}': {e}\n")
            sys.exit(EXIT_IO_ERROR)
    else:
        sys.stdout.write(output_str + "\n")
        sys.stdout.flush()


def cmd_stats(args: argparse.Namespace) -> int:
    records, headers = read_dataset(args.input, args.verbose)
    if not records:
        write_output({"total_rows": 0, "columns": {}}, args.output, args.format, args.round)
        return EXIT_SUCCESS

    col_stats: Dict[str, Any] = {}
    for h in headers:
        values = [r.get(h) for r in records if r.get(h) is not None]
        numeric_vals = [float(v) for v in values if isinstance(v, (int, float)) and not isinstance(v, bool)]
        null_count = len(records) - len(values)


        stat: Dict[str, Any] = {
            "count": len(values),
            "null_count": null_count,
            "type": "numeric" if len(numeric_vals) == len(values) and len(values) > 0 else "text",
        }

        if numeric_vals:
            sorted_nums = sorted(numeric_vals)
            n = len(sorted_nums)
            total = sum(sorted_nums)
            mean = total / n
            min_val = sorted_nums[0]
            max_val = sorted_nums[-1]
            median = sorted_nums[n // 2] if n % 2 != 0 else (sorted_nums[n // 2 - 1] + sorted_nums[n // 2]) / 2.0
            
            variance = sum((x - mean) ** 2 for x in sorted_nums) / n
            stddev = variance ** 0.5

            stat.update({
                "min": min_val,
                "max": max_val,
                "sum": total,
                "mean": round(mean, args.round) if args.round is not None else mean,
                "median": round(median, args.round) if args.round is not None else median,
                "stddev": round(stddev, args.round) if args.round is not None else stddev,
            })

        col_stats[h] = stat

    result = {
        "total_rows": len(records),
        "total_columns": len(headers),
        "columns": col_stats,
    }
    write_output(result, args.output, args.format, args.round)
    return EXIT_SUCCESS


def cmd_filter(args: argparse.Namespace) -> int:
    records, _ = read_dataset(args.input, args.verbose)
    
    # Parse condition e.g. "age >= 30" or "status == active"
    filtered = records
    if args.condition:
        cond = args.condition.strip()
        parts = None
        op = None
        for operator in [">=", "<=", "!=", "==", ">", "<", "="]:
            if operator in cond:
                op = operator
                idx = cond.index(operator)
                left = cond[:idx].strip()
                right = cond[idx + len(operator):].strip().strip("\"'")
                parts = (left, op, right)
                break
        
        if not parts:
            sys.stderr.write(f"Syntax error in filter condition: '{args.condition}'. Expected format: 'col >= val'\n")
            return EXIT_ARG_ERROR
        
        col, op, raw_val = parts
        
        # Try numeric cast
        target_val: Any = raw_val
        try:
            if "." in raw_val:
                target_val = float(raw_val)
            else:
                target_val = int(raw_val)
        except ValueError:
            pass

        def matches(r: Dict[str, Any]) -> bool:
            v = r.get(col)
            if v is None:
                return False
            try:
                if op in ("==", "="):
                    return str(v).lower() == str(target_val).lower() if isinstance(target_val, str) else v == target_val
                elif op == "!=":
                    return v != target_val
                elif op == ">":
                    return float(v) > float(target_val)
                elif op == ">=":
                    return float(v) >= float(target_val)
                elif op == "<":
                    return float(v) < float(target_val)
                elif op == "<=":
                    return float(v) <= float(target_val)
            except (ValueError, TypeError):
                return False
            return False

        filtered = [r for r in filtered if matches(r)]

    if args.sort_by:
        sort_col = args.sort_by
        def sort_key(r: Dict[str, Any]) -> Any:
            val = r.get(sort_col)
            if val is None:
                return ""
            return val
        try:
            filtered.sort(key=sort_key, reverse=args.descending)
        except TypeError:
            # Fallback to string sorting on mixed types
            filtered.sort(key=lambda r: str(r.get(sort_col, "")), reverse=args.descending)

    if args.limit is not None and args.limit >= 0:
        filtered = filtered[:args.limit]

    write_output(filtered, args.output, args.format)
    return EXIT_SUCCESS


def cmd_transform(args: argparse.Namespace) -> int:
    records, _ = read_dataset(args.input, args.verbose)
    
    transformed: List[Dict[str, Any]] = []
    for r in records:
        new_row = dict(r)
        
        # Case transform
        if args.upper_col and args.upper_col in new_row:
            val = new_row[args.upper_col]
            if val is not None:
                new_row[args.upper_col] = str(val).upper()

        if args.lower_col and args.lower_col in new_row:
            val = new_row[args.lower_col]
            if val is not None:
                new_row[args.lower_col] = str(val).lower()

        # Scale numeric column
        if args.scale_col and args.scale_factor is not None and args.scale_col in new_row:
            val = new_row[args.scale_col]
            if isinstance(val, (int, float)):
                new_row[args.scale_col] = val * args.scale_factor

        # Cryptographic hash column
        if args.hash_col and args.hash_col in new_row:
            val = new_row[args.hash_col]
            if val is not None:
                digest = hashlib.sha256(str(val).encode("utf-8")).hexdigest()
                new_row[f"{args.hash_col}_sha256"] = digest

        transformed.append(new_row)

    write_output(transformed, args.output, args.format)
    return EXIT_SUCCESS


def cmd_inspect(args: argparse.Namespace) -> int:
    records, headers = read_dataset(args.input, args.verbose)
    
    size_bytes = 0
    if args.input and args.input != "-" and os.path.exists(args.input):
        size_bytes = os.path.getsize(args.input)

    types: Dict[str, str] = {}
    for h in headers:
        sample_vals = [r.get(h) for r in records if r.get(h) is not None]
        if not sample_vals:
            types[h] = "null"
        elif all(isinstance(v, bool) for v in sample_vals):
            types[h] = "boolean"
        elif all(isinstance(v, (int, float)) and not isinstance(v, bool) for v in sample_vals):
            types[h] = "number"
        else:
            types[h] = "string"


    inspection = {
        "version": VERSION,
        "source": args.input if args.input else "stdin",
        "row_count": len(records),
        "column_count": len(headers),
        "columns": headers,
        "types": types,
        "file_size_bytes": size_bytes,
    }
    write_output(inspection, args.output, args.format)
    return EXIT_SUCCESS


def crunch_record(record: Dict[str, Any], rounds: int) -> Tuple[str, float]:
    # Deterministic string representation of record keys/values
    serialized = "|".join(f"{k}:{v}" for k, v in sorted(record.items()))
    digest = serialized
    for _ in range(rounds):
        digest = hashlib.sha256(digest.encode("utf-8")).hexdigest()

    score = 0.0
    for v in record.values():
        if isinstance(v, (int, float)) and not isinstance(v, bool):
            score += float(v)
    for _ in range(rounds):
        score = (score * 1.0001) + 0.005
    return digest, score


def crunch_chunk(args_tuple: Tuple[List[Dict[str, Any]], int]) -> List[Tuple[str, float]]:
    chunk, rounds = args_tuple
    return [crunch_record(r, rounds) for r in chunk]


def cmd_crunch(args: argparse.Namespace) -> int:
    t0 = time.perf_counter()
    records, _ = read_dataset(args.input, args.verbose)
    if not records:
        sys.stderr.write("Error: empty dataset provided for crunch\n")
        sys.exit(EXIT_DATA_ERROR)

    workers = args.workers if args.workers and args.workers > 0 else max(1, mp.cpu_count())
    chunk_size = max(1, args.chunk_size)
    rounds = max(1, args.rounds)

    chunks = [records[i : i + chunk_size] for i in range(0, len(records), chunk_size)]
    tasks = [(c, rounds) for c in chunks]

    results: List[Tuple[str, float]] = []

    if args.mode == "sequential" or len(records) <= 1:
        for t in tasks:
            results.extend(crunch_chunk(t))
    elif args.mode == "multiprocessing":
        with mp.Pool(processes=workers) as pool:
            chunk_results = pool.map(crunch_chunk, tasks)
            for cr in chunk_results:
                results.extend(cr)
    elif args.mode == "threading":
        with ThreadPoolExecutor(max_workers=workers) as executor:
            chunk_results = list(executor.map(crunch_chunk, tasks))
            for cr in chunk_results:
                results.extend(cr)
    elif args.mode == "async":
        async def run_async():
            loop = asyncio.get_running_loop()
            async_tasks = [loop.run_in_executor(None, crunch_chunk, t) for t in tasks]
            return await asyncio.gather(*async_tasks)

        chunk_results = asyncio.run(run_async())
        for cr in chunk_results:
            results.extend(cr)
    else:
        sys.stderr.write(f"Unknown concurrency mode: {args.mode}\n")
        sys.exit(EXIT_ARG_ERROR)

    # Compute aggregate checksum & composite score
    hasher = hashlib.sha256()
    total_score = 0.0
    for d, s in results:
        hasher.update(d.encode("utf-8"))
        total_score += s

    aggregate_checksum = hasher.hexdigest()
    composite_mean = round(total_score / len(results), 2)
    elapsed_ms = (time.perf_counter() - t0) * 1000.0

    if args.benchmark:
        sys.stderr.write(
            f"[dataforge:perf] mode={args.mode} workers={workers} records={len(records)} rounds={rounds} elapsed_ms={elapsed_ms:.2f}\n"
        )

    summary = {
        "command": "crunch",
        "mode": args.mode,
        "workers": workers,
        "total_records": len(records),
        "total_work_units": len(records) * rounds,
        "composite_mean": composite_mean,
        "aggregate_checksum": aggregate_checksum,
    }

    write_output(summary, args.output, args.format)
    return EXIT_SUCCESS


def build_parser() -> argparse.ArgumentParser:
    default_format = os.environ.get("DATAFORGE_DEFAULT_FORMAT", "json")
    if default_format not in ("json", "csv", "summary"):
        default_format = "json"

    parser = argparse.ArgumentParser(
        prog="dataforge",
        description="High-performance data manipulation, filtering, and summary analytics CLI utility.",
    )
    parser.add_argument("--version", action="version", version=f"%(prog)s {VERSION}")
    parser.add_argument("-v", "--verbose", action="store_true", help="Enable verbose debug logs to stderr")
    parser.add_argument("-q", "--quiet", action="store_true", help="Suppress non-error logs")

    subparsers = parser.add_subparsers(dest="subcommand", title="Available Commands", required=True)

    # Subcommand: stats
    p_stats = subparsers.add_parser("stats", help="Compute numerical and distribution statistics")
    p_stats.add_argument("input", nargs="?", default="-", help="Input file path or '-' for stdin (default: -)")
    p_stats.add_argument("-o", "--output", help="Output file path (default: stdout)")
    p_stats.add_argument("-f", "--format", choices=["json", "csv"], default=default_format, help=f"Output format (default: {default_format})")
    p_stats.add_argument("-r", "--round", type=int, default=2, help="Decimal places to round numeric stats (default: 2)")

    # Subcommand: filter
    p_filter = subparsers.add_parser("filter", help="Filter and sort rows by predicate expression")
    p_filter.add_argument("input", nargs="?", default="-", help="Input file path or '-' for stdin (default: -)")
    p_filter.add_argument("-c", "--condition", help="Filter condition, e.g. 'age >= 30' or 'status == active'")
    p_filter.add_argument("-s", "--sort-by", help="Sort rows by specified column name")
    p_filter.add_argument("-d", "--descending", action="store_true", help="Sort in descending order")
    p_filter.add_argument("-l", "--limit", type=int, help="Maximum number of rows to return")
    p_filter.add_argument("-o", "--output", help="Output file path (default: stdout)")
    p_filter.add_argument("-f", "--format", choices=["json", "csv"], default=default_format, help=f"Output format (default: {default_format})")

    # Subcommand: transform
    p_trans = subparsers.add_parser("transform", help="Apply transformations to columns")
    p_trans.add_argument("input", nargs="?", default="-", help="Input file path or '-' for stdin (default: -)")
    p_trans.add_argument("--upper-col", help="Convert text in specified column to UPPERCASE")
    p_trans.add_argument("--lower-col", help="Convert text in specified column to lowercase")
    p_trans.add_argument("--scale-col", help="Multiply values in numeric column by factor")
    p_trans.add_argument("--scale-factor", type=float, help="Scaling factor multiplier")
    p_trans.add_argument("--hash-col", help="Compute SHA-256 hash for values in column")
    p_trans.add_argument("-o", "--output", help="Output file path (default: stdout)")
    p_trans.add_argument("-f", "--format", choices=["json", "csv"], default=default_format, help=f"Output format (default: {default_format})")

    # Subcommand: inspect
    p_inspect = subparsers.add_parser("inspect", help="Inspect schema, inferred types, and row count")
    p_inspect.add_argument("input", nargs="?", default="-", help="Input file path or '-' for stdin (default: -)")
    p_inspect.add_argument("-o", "--output", help="Output file path (default: stdout)")
    p_inspect.add_argument("-f", "--format", choices=["json"], default="json", help="Output format (default: json)")

    # Subcommand: crunch (heavy CPU concurrency)
    p_crunch = subparsers.add_parser("crunch", help="Perform heavy multi-process/multi-thread/async CPU analytics")
    p_crunch.add_argument("input", nargs="?", default="-", help="Input file path or '-' for stdin (default: -)")
    p_crunch.add_argument("-w", "--workers", type=int, default=4, help="Number of worker processes/threads (default: 4)")
    p_crunch.add_argument("-m", "--mode", choices=["multiprocessing", "threading", "async", "sequential"], default="multiprocessing", help="Concurrency execution model (default: multiprocessing)")
    p_crunch.add_argument("-c", "--chunk-size", type=int, default=50, help="Record chunk size for worker dispatch (default: 50)")
    p_crunch.add_argument("-r", "--rounds", type=int, default=50, help="CPU workload rounds per record (default: 50)")
    p_crunch.add_argument("--benchmark", action="store_true", help="Log execution timing to stderr")
    p_crunch.add_argument("-o", "--output", help="Output file path (default: stdout)")
    p_crunch.add_argument("-f", "--format", choices=["json", "csv"], default=default_format, help=f"Output format (default: {default_format})")

    return parser


def main() -> None:
    parser = build_parser()
    try:
        args = parser.parse_args()
    except SystemExit as e:
        # Exit code 2 from argparse is argument syntax error -> map to EXIT_ARG_ERROR (1) or exit code
        code = e.code if isinstance(e.code, int) else EXIT_ARG_ERROR
        sys.exit(EXIT_ARG_ERROR if code != 0 else EXIT_SUCCESS)

    if args.subcommand == "stats":
        sys.exit(cmd_stats(args))
    elif args.subcommand == "filter":
        sys.exit(cmd_filter(args))
    elif args.subcommand == "transform":
        sys.exit(cmd_transform(args))
    elif args.subcommand == "inspect":
        sys.exit(cmd_inspect(args))
    elif args.subcommand == "crunch":
        sys.exit(cmd_crunch(args))
    else:
        sys.stderr.write(f"Unknown subcommand: {args.subcommand}\n")
        sys.exit(EXIT_ARG_ERROR)


if __name__ == "__main__":
    main()
