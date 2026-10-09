"""Independent installed MCP SDK client; no hosted UI compatibility claim."""
import asyncio, hashlib, json, pathlib, sys
from datetime import timedelta
from importlib.metadata import version
from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client

def snapshot(root):
    return {str(p.relative_to(root)): hashlib.sha256(p.read_bytes()).hexdigest()
            for p in root.rglob('*') if p.is_file()}

async def main():
    assert version('mcp') == '1.29.0'
    binary, root, digest = sys.argv[1:]
    root = pathlib.Path(root)
    params = StdioServerParameters(command=binary, args=['--local-readonly-v1', str(root), 'controller.json', digest])
    async with stdio_client(params) as (read, write):
        async with ClientSession(read, write, read_timeout_seconds=timedelta(seconds=15)) as session:
            init = await session.initialize()
            assert init.protocolVersion == '2025-11-25'
            tools = await session.list_tools()
            assert len(tools.tools) == 1 and tools.tools[0].annotations.readOnlyHint
            before = snapshot(root)
            result = await session.call_tool('flowguard_read', {'job':'discover','requestKey':'sdk'})
            assert result.structuredContent['exitCode'] == 0
            assert len(result.structuredContent['bundle']['inventory']['sources']) == 10
            assert before == snapshot(root)
            denied = await session.call_tool('flowguard_read', {'job':'discover','requestKey':'denied','actorVerified':True})
            assert denied.isError and denied.structuredContent['exitCode'] == 4
            # External setup mutation is not reachable via the host interface.
            (root/'docs/project/02-architecture.md').unlink()
            changed = snapshot(root)
            later = await session.call_tool('flowguard_read', {'job':'discover','requestKey':'sdk'})
            assert later.structuredContent['exitCode'] == 2
            assert changed == snapshot(root)
    print(json.dumps({'client':'mcp-python/1.29.0','protocol':init.protocolVersion,'repeatedReadExit':2,'sourceReadOnly':True}))

asyncio.run(main())
