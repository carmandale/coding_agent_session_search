import sys
from pathlib import Path


def main() -> int:
    workspace = Path(sys.argv[1]) if len(sys.argv) > 1 else Path.cwd()
    
    if not workspace.exists():
        print(f"Workspace not found: {workspace}")
        return 1
    
    from cass_tui.app import CassTuiApp
    
    app = CassTuiApp(workspace=workspace)
    app.run()
    return 0


if __name__ == "__main__":
    sys.exit(main())
