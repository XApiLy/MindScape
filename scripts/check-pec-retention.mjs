import { readdir, stat } from "node:fs/promises";
import path from "node:path";
import process from "node:process";

const root = process.cwd();
const currentDir = path.join(root, "Project Engineering Communication");
const archiveDir = path.join(root, "Project Engineering Communication Archive");
const employeePattern = /^员工(0[1-6])-.+-\d{8}-\d{4}\.md$/u;
const controlPattern = /^(?:PEC-.+|任务派发-.+-\d{8}-\d{4})\.md$/u;
const errors = [];

let entries = [];
try {
  entries = await readdir(currentDir, { withFileTypes: true });
} catch (error) {
  errors.push(`无法读取当前 PEC 目录：${error.message}`);
}

try {
  if (!(await stat(archiveDir)).isDirectory()) {
    errors.push("PEC 归档路径存在，但不是目录。");
  }
} catch {
  errors.push("缺少独立归档目录：Project Engineering Communication Archive/");
}

const files = entries.filter((entry) => entry.isFile()).map((entry) => entry.name).sort();
const nested = entries.filter((entry) => entry.isDirectory()).map((entry) => entry.name).sort();

if (nested.length > 0) {
  errors.push(`当前 PEC 目录禁止包含子目录：${nested.join("、")}`);
}

if (files.length !== 7) {
  errors.push(`当前 PEC 目录必须恰好有 7 个文件，实际为 ${files.length} 个。`);
}

const employeeCounts = new Map(Array.from({ length: 6 }, (_, index) => [`0${index + 1}`, 0]));
const controlFiles = [];

for (const file of files) {
  if (!file.endsWith(".md")) {
    errors.push(`当前 PEC 目录只能包含 Markdown：${file}`);
    continue;
  }

  const employeeMatch = file.match(employeePattern);
  if (employeeMatch) {
    employeeCounts.set(employeeMatch[1], employeeCounts.get(employeeMatch[1]) + 1);
  } else {
    controlFiles.push(file);
  }
}

for (const [employee, count] of employeeCounts) {
  if (count !== 1) {
    errors.push(`员工${employee}必须且只能有 1 份当前 PEC，实际为 ${count} 份。`);
  }
}

if (controlFiles.length !== 1) {
  errors.push(`必须且只能有 1 份规则或任务派发文件，实际为 ${controlFiles.length} 份。`);
} else if (!controlPattern.test(controlFiles[0])) {
  errors.push(`第七文件命名不符合规则：${controlFiles[0]}`);
}

if (errors.length > 0) {
  console.error("PEC 当前窗口校验失败：");
  for (const error of errors) console.error(`- ${error}`);
  process.exitCode = 1;
} else {
  console.log("PEC 当前窗口校验通过：员工01～06各一份最新报告，另有一份控制文件。");
  for (const file of files) console.log(`- ${file}`);
}
