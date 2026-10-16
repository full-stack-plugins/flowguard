"""只读复用 FlowGuard 阶段事实源；不执行阶段推进或用户审批。"""
import json
import hashlib
import os
from pathlib import Path
import stat
import sys

sys.path.insert(0, str(Path(sys.argv[1]) / 'scripts'))
result = {'decision': 'unverified', 'approval': 'required', 'quality_decision': 'not_evaluated'}
try:
    from flowguard_lib import stage_docs, registry
    request = json.loads(sys.argv[3])
    root = Path(sys.argv[2])
    # 独立适配层限制读取范围，不改变原插件的阶段或审批语义。
    limit = 1024 * 1024
    total_limit = 16 * limit
    sources = {}
    total_bytes = 0

    def checked_path(path):
        path = Path(path).absolute()
        path.relative_to(root.absolute())
        for ancestor in reversed((path, *path.parents)):
            try:
                info = ancestor.lstat()
            except FileNotFoundError:
                continue
            if stat.S_ISLNK(info.st_mode) or getattr(info, 'st_file_attributes', 0) & getattr(stat, 'FILE_ATTRIBUTE_REPARSE_POINT', 0x400):
                raise ValueError('linked stage source')
            if ancestor != path and not stat.S_ISDIR(info.st_mode):
                raise ValueError('non-directory stage ancestor')
        path.resolve().relative_to(root.resolve())
        return path

    def source_bytes(path):
        path = checked_path(path)
        try:
            before = path.lstat()
        except FileNotFoundError:
            return None
        if not stat.S_ISREG(before.st_mode) or before.st_size > limit:
            raise ValueError('stage source is not a bounded regular file')
        with path.open('rb') as handle:
            opened = os.fstat(handle.fileno())
            if not stat.S_ISREG(opened.st_mode) or opened.st_size > limit:
                raise ValueError('stage source changed before open')
            data = handle.read(limit + 1)
        after = checked_path(path).lstat()
        identity = lambda value: (value.st_dev, value.st_ino, value.st_size, value.st_mtime_ns)
        if len(data) > limit or identity(before) != identity(opened) or identity(opened) != identity(after):
            raise ValueError('stage source changed during read')
        return data

    def remember(path):
        global total_bytes
        path = checked_path(path)
        if path not in sources:
            data = source_bytes(path)
            total_bytes += len(data) if data is not None else 0
            if total_bytes > total_limit:
                raise ValueError('total stage sources exceed limit')
            sources[path] = data
        return sources[path]

    # 原生注册表定位十阶段，先检查全部输入；不存在的文件仍由原生语义处理。
    for stage in registry.ARTIFACTS:
        remember(stage_docs.path_for(root, request['task_id'], stage))

    def guarded_read_text(path, encoding=None, errors=None, **kwargs):
        data = remember(path)
        if data is None:
            raise FileNotFoundError(str(path))
        # 对原生服务读取的跨功能发布依赖也统一应用边界与缓存。
        return data.decode(encoding or 'utf-8', errors or 'strict').replace('\r\n', '\n').replace('\r', '\n')

    Path.read_text = guarded_read_text
    snapshot = stage_docs.snapshot(root, request['task_id'])
    result['snapshot'] = snapshot
    if request.get('mode', 'check') == 'status':
        result.update(decision='observed', code='stage_status_observed')
    else:
        missing = stage_docs.missing_before(root, request['task_id'], request['action'])
        result.update(decision='deny' if missing else 'requires_approval', missing=missing,
                      code='stage_prerequisites_missing' if missing else 'stage_artifacts_ready_approval_required')
    # 使用同一输入快照评估；检查后重新读取摘要，变化时丢弃结果。
    for path, original in sources.items():
        current = source_bytes(path)
        if (original is None) != (current is None) or (original is not None and hashlib.sha256(original).digest() != hashlib.sha256(current).digest()):
            raise ValueError('stage source changed during evaluation')
    result['envelope'] = {'severity': 'ERROR' if result['decision'] == 'deny' else 'INFO',
                          'code': result['code'], 'message': '只读阶段产物预检；不授予写入、提交或发布授权。',
                          'fix': '补齐原生阶段产物并经真实用户审批；提交/发布前另行核验当前证据。'}
except Exception:
    result = {'decision': 'unverified', 'approval': 'required', 'quality_decision': 'not_evaluated',
              'code': 'stage_adapter_unverified', 'envelope': {'severity': 'ERROR', 'code': 'stage_adapter_unverified',
              'message': '阶段来源越界、超限、变化、文档非法或可信适配器不兼容。',
              'fix': '使用项目内普通阶段文件（单文件不超过1MiB、总量不超过16MiB），修复元信息后复验。'}}
print(json.dumps(result, ensure_ascii=False))
