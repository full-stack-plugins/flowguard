"""固定真实 OPA 的 Rego 测试、包摘要检查与独立验收表差分。不下载或安装工具。"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile

from policy_gate_cases import cases


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--opa", type=Path)
    parser.add_argument("--scratch", type=Path)
    parser.add_argument("--export-cases", type=Path, help="只导出显式验收表，供通用执行器独立差分")
    args = parser.parse_args()
    if args.export_cases:
        args.export_cases.write_text(json.dumps(cases(), ensure_ascii=False, indent=2) + "\n", encoding="utf-8", newline="\n")
        print(f"Exported {len(cases())} explicit reference cases")
        return
    if not args.opa or not args.scratch:
        parser.error("verification requires --opa and --scratch")
    package = Path(__file__).resolve().parents[1] / "policy/engineering-gate/v1"
    executable = args.opa.resolve(strict=True)
    version = subprocess.run([str(executable), "version"], check=True,
                             capture_output=True, text=True, timeout=10)
    assert version.stdout.splitlines().count("Version: 1.21.2") == 1, version.stdout
    manifest = json.loads((package / "manifest.json").read_text(encoding="utf-8"))
    for item in manifest["files"] + [manifest["capabilities"]]:
        raw = (package / item["path"]).read_bytes()
        assert b"\r" not in raw, "package must retain locked LF bytes"
        assert "sha256:" + hashlib.sha256(raw).hexdigest() == item["digest"], item["path"]
    subprocess.run([str(executable), "check", "--strict", "--capabilities", str(package / "capabilities.json"),
                    str(package / "gate.rego")], check=True, timeout=20)
    subprocess.run([str(executable), "test", str(package), "-v"], check=True, timeout=30)
    expected_fields = {"schemaVersion", "decision", "reasons", "requiredActions", "missingObligations", "activeViolations"}
    # 与真实通用执行器相同入口/能力文件，独立表给出期望，绝不以当前输出生成期望。
    with tempfile.TemporaryDirectory(prefix="flow-policy-", dir=args.scratch.resolve(strict=True)) as scratch:
        fixture = Path(scratch) / "input.json"
        for case in cases():
            fixture.write_text(json.dumps(case["input"], ensure_ascii=False), encoding="utf-8")
            evaluated = subprocess.run([str(executable), "eval", "--format=json", "--strict-builtin-errors",
                                        "--capabilities", str(package / "capabilities.json"),
                                        "--data", str(package / "gate.rego"), "--input", str(fixture),
                                        manifest["entrypoint"]], check=True, capture_output=True, text=True, timeout=10)
            output = json.loads(evaluated.stdout)["result"][0]["expressions"][0]["value"]
            assert set(output) == expected_fields, case["name"]
            assert output["schemaVersion"] == 1, case["name"]
            assert output["decision"] == case["expectedDecision"], (case["name"], output)
            assert len(output["missingObligations"]) == case["expectedMissing"], (case["name"], output)
            assert len(output["activeViolations"]) == case["expectedFindings"], (case["name"], output)
    print(f"PASS: {len(cases())} independent decision-table evaluations; package raw hashes and pure capabilities verified")


if __name__ == "__main__":
    main()
