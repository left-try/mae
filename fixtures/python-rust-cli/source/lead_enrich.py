#!/usr/bin/env python3
"""Lead sourcing and enrichment deterministic CLI pipeline slice."""

import argparse
import hashlib
import json
import os
import sys

PROG = "lead_enrich"
VERSION = "1.2.0"

EXIT_SUCCESS = 0
EXIT_ARG_ERROR = 1
EXIT_IO_ERROR = 2
EXIT_DATA_ERROR = 3
EXIT_STAGE_ERROR = 4


class CliParser(argparse.ArgumentParser):
    def error(self, message):
        sys.stderr.write(f"ERROR [ARG]: {message}\n")
        sys.exit(EXIT_ARG_ERROR)


def build_parser():
    parser = CliParser(
        prog="lead_enrich",
        description="Lead sourcing and enrichment deterministic CLI pipeline slice",
    )
    parser.add_argument("--version", action="version", version=f"{PROG} {VERSION}")
    parser.add_argument("-v", "--verbose", action="store_true", help="Enable verbose logging")
    parser.add_argument("-q", "--quiet", action="store_true", help="Suppress non-error stderr logs")

    subparsers = parser.add_subparsers(dest="subcommand")

    p_ingest = subparsers.add_parser("ingest", help="Validate, normalize, and deduplicate raw leads")
    p_ingest.add_argument("input", help="Input raw leads JSON path")
    p_ingest.add_argument("-o", "--output", default="", help="Optional output file path for normalized leads")
    p_ingest.add_argument("--dedupe", action="store_true", help="Deduplicate leads by normalized email")

    p_enrich = subparsers.add_parser("enrich", help="Enrich leads with domain cache, retry policy, and resume")
    p_enrich.add_argument("input", help="Input leads JSON path")
    p_enrich.add_argument("-s", "--state-file", default="", help="Path to persistent state and cache ledger JSON")
    p_enrich.add_argument("-o", "--output", default="", help="Optional output file path for enriched leads")
    p_enrich.add_argument("-r", "--max-retries", type=int, default=2, help="Maximum retry attempts per domain")
    p_enrich.add_argument("-f", "--fail-domain", default="", help="Domain that triggers transient provider failures")
    p_enrich.add_argument("--resume", action="store_true", help="Resume from existing state ledger skipping completed leads")

    p_report = subparsers.add_parser("report", help="Inspect state ledger and emit stage outcome report")
    p_report.add_argument("state_file", help="Path to persistent state ledger JSON")
    p_report.add_argument("-f", "--format", choices=["json", "summary"], default="json", help="Report output format")

    return parser


def load_json_file(path, label="input"):
    if not path or not os.path.exists(path):
        sys.stderr.write(f"ERROR [IO]: {label} file not found: {path}\n")
        sys.exit(EXIT_IO_ERROR)
    try:
        with open(path, "r", encoding="utf-8") as f:
            return json.load(f)
    except Exception as exc:
        sys.stderr.write(f"ERROR [DATA]: failed to parse {label} JSON '{path}': {exc}\n")
        sys.exit(EXIT_DATA_ERROR)


def write_json_atomic(path, payload):
    parent = os.path.dirname(path)
    if parent:
        os.makedirs(parent, exist_ok=True)
    with open(path, "w", encoding="utf-8") as f:
        json.dump(payload, f, indent=2, sort_keys=True)
        f.write("\n")


def normalize_leads(raw_records, dedupe=False):
    if not isinstance(raw_records, list):
        sys.stderr.write("ERROR [DATA]: expected JSON array of lead records\n")
        sys.exit(EXIT_DATA_ERROR)

    normalized = []
    seen_emails = set()
    duplicates_removed = 0

    for idx, rec in enumerate(raw_records):
        if not isinstance(rec, dict):
            sys.stderr.write(f"ERROR [DATA]: record at index {idx} is not an object\n")
            sys.exit(EXIT_DATA_ERROR)
        lead_id = str(rec.get("id", "")).strip()
        email = str(rec.get("email", "")).strip().lower()
        company = str(rec.get("company", "")).strip()
        role = str(rec.get("role", "")).strip()

        if not lead_id or not email or "@" not in email:
            sys.stderr.write(f"ERROR [DATA]: invalid lead record at index {idx} (id='{lead_id}', email='{email}')\n")
            sys.exit(EXIT_DATA_ERROR)

        parts = email.split("@", 1)
        domain = parts[1].strip()
        if not domain or "." not in domain:
            sys.stderr.write(f"ERROR [DATA]: invalid email domain at index {idx}: '{email}'\n")
            sys.exit(EXIT_DATA_ERROR)

        if dedupe:
            if email in seen_emails:
                duplicates_removed += 1
                continue
            seen_emails.add(email)

        normalized.append({
            "company": company,
            "domain": domain,
            "email": email,
            "id": lead_id,
            "role": role,
        })

    return normalized, duplicates_removed


def deterministic_domain_profile(domain):
    digest = hashlib.sha256(domain.encode("utf-8")).hexdigest()
    score = (int(digest[:4], 16) % 50) + 50
    tier = "enterprise" if score >= 80 else ("mid_market" if score >= 65 else "growth")
    return {
        "domain": domain,
        "employee_band": "250-1000" if tier == "enterprise" else "50-250",
        "enrichment_hash": digest[:12],
        "lead_score": score,
        "tier": tier,
    }


def cmd_ingest(args):
    raw_records = load_json_file(args.input, "input")
    normalized, duplicates_removed = normalize_leads(raw_records, dedupe=args.dedupe)
    result = {
        "duplicates_removed": duplicates_removed,
        "leads": normalized,
        "stage": "ingest",
        "total_input": len(raw_records),
        "valid_count": len(normalized),
    }
    if args.output:
        write_json_atomic(args.output, result)
    sys.stdout.write(json.dumps(result, indent=2, sort_keys=True) + "\n")
    return EXIT_SUCCESS


def cmd_enrich(args):
    if args.max_retries < 0:
        sys.stderr.write("ERROR [ARG]: --max-retries must be non-negative\n")
        return EXIT_ARG_ERROR

    raw_data = load_json_file(args.input, "input")
    if isinstance(raw_data, dict) and "leads" in raw_data:
        leads, _ = normalize_leads(raw_data["leads"], dedupe=False)
    else:
        leads, _ = normalize_leads(raw_data, dedupe=False)

    state = {
        "cache_hits": 0,
        "completed_ids": [],
        "domain_cache": {},
        "enriched_leads": [],
        "failed_ids": [],
        "resumed_skipped": 0,
        "retried_count": 0,
        "stage_status": "in_progress",
    }

    if args.resume and args.state_file and os.path.exists(args.state_file):
        prior = load_json_file(args.state_file, "state")
        if isinstance(prior, dict):
            state["domain_cache"] = dict(prior.get("domain_cache", {}))
            state["completed_ids"] = list(prior.get("completed_ids", []))
            state["enriched_leads"] = list(prior.get("enriched_leads", []))

    completed_set = set(state["completed_ids"])

    for lead in leads:
        lead_id = lead["id"]
        domain = lead["domain"]

        if args.resume and lead_id in completed_set:
            state["resumed_skipped"] += 1
            continue

        if domain in state["domain_cache"]:
            profile = state["domain_cache"][domain]
            state["cache_hits"] += 1
        else:
            # Check if domain triggers transient provider failure simulation
            if args.fail_domain and domain == args.fail_domain:
                attempts = 0
                succeeded = False
                while attempts <= args.max_retries:
                    attempts += 1
                    if attempts >= 3:
                        succeeded = True
                        break
                    state["retried_count"] += 1
                    sys.stderr.write(
                        f"WARN [RETRY]: transient provider failure for domain='{domain}' attempt={attempts}/{args.max_retries + 1}\n"
                    )
                if not succeeded:
                    state["failed_ids"].append(lead_id)
                    state["stage_status"] = "failed"
                    if args.state_file:
                        write_json_atomic(args.state_file, state)
                    sys.stderr.write(
                        f"ERROR [STAGE]: retries exhausted for domain='{domain}' lead_id='{lead_id}'\n"
                    )
                    return EXIT_STAGE_ERROR

            profile = deterministic_domain_profile(domain)
            state["domain_cache"][domain] = profile

        enriched_record = {
            "company": lead["company"],
            "domain": domain,
            "email": lead["email"],
            "enrichment_hash": profile["enrichment_hash"],
            "id": lead_id,
            "lead_score": profile["lead_score"],
            "role": lead["role"],
            "tier": profile["tier"],
        }
        state["enriched_leads"].append(enriched_record)
        state["completed_ids"].append(lead_id)
        completed_set.add(lead_id)

    state["stage_status"] = "completed"
    if args.state_file:
        write_json_atomic(args.state_file, state)

    output_payload = {
        "cache_hits": state["cache_hits"],
        "completed_count": len(state["completed_ids"]),
        "enriched_leads": state["enriched_leads"],
        "failed_count": len(state["failed_ids"]),
        "resumed_skipped": state["resumed_skipped"],
        "retried_count": state["retried_count"],
        "stage": "enrich",
        "stage_status": state["stage_status"],
    }
    if args.output:
        write_json_atomic(args.output, output_payload)

    sys.stdout.write(json.dumps(output_payload, indent=2, sort_keys=True) + "\n")
    return EXIT_SUCCESS


def cmd_report(args):
    state = load_json_file(args.state_file, "state")
    completed_count = len(state.get("completed_ids", []))
    failed_count = len(state.get("failed_ids", []))
    cache_hits = int(state.get("cache_hits", 0))
    retried_count = int(state.get("retried_count", 0))
    resumed_skipped = int(state.get("resumed_skipped", 0))
    stage_status = str(state.get("stage_status", "unknown"))

    if args.format == "summary":
        sys.stdout.write(
            f"STAGE_REPORT status={stage_status} completed={completed_count} failed={failed_count} "
            f"cache_hits={cache_hits} retries={retried_count} resumed_skipped={resumed_skipped}\n"
        )
    else:
        report = {
            "cache_hits": cache_hits,
            "completed_count": completed_count,
            "failed_count": failed_count,
            "resumed_skipped": resumed_skipped,
            "retried_count": retried_count,
            "stage": "report",
            "stage_status": stage_status,
        }
        sys.stdout.write(json.dumps(report, indent=2, sort_keys=True) + "\n")
    return EXIT_SUCCESS


def main():
    parser = build_parser()
    args = parser.parse_args()

    if not args.subcommand:
        sys.stderr.write("ERROR [ARG]: missing required subcommand (choose from: ingest, enrich, report)\n")
        sys.exit(EXIT_ARG_ERROR)

    if args.subcommand == "ingest":
        sys.exit(cmd_ingest(args))
    elif args.subcommand == "enrich":
        sys.exit(cmd_enrich(args))
    elif args.subcommand == "report":
        sys.exit(cmd_report(args))
    else:
        sys.stderr.write(f"ERROR [ARG]: unknown subcommand '{args.subcommand}'\n")
        sys.exit(EXIT_ARG_ERROR)


if __name__ == "__main__":
    main()
