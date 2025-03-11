# [kibitz](https://github.com/nick1udwig/kibitz) on [hyperware](https://github.com/hyperware-ai/hyperdrive)

Chat with LLMs and use tools, deployed on Hyperware

## Installation

Go to the Hyperware App Store and download & install kibitz.

## Configuration

### Run WS-MCP

For a [locally hosted node](https://book.hyperware.ai/getting_started/install.html), run a [ws-mcp server](https://github.com/nick1udwig/ws-mcp) alongside to enable tool use:
```bash
# Install uv python manager
curl -LsSf https://astral.sh/uv/install.sh | sh

# Run the WS-MCP server
uvx --refresh ws-mcp@latest
```

See [ws-mcp docs](https://github.com/nick1udwig/ws-mcp) for more details.

### For mobile access

You can use kibitz to run tools on your laptop/desktop computer from your mobile device!
This allows for remote code editing, and more.

1. [Set up a locally hosted node and a ws-mcp server](#run-ws-mcp).
2. [Get a hosted node](https://valet.hyperware.ai/), or else run a node on a VPS or somewhere you can access from mobile.
3. Run your desktop ws-mcp server.
4. Install kibitz on both of the nodes.
5. On each node, configure fwd-ws, an app that comes bundled with kibitz.
   You can access it from your node's homepage `Show Apps` section.
   You will need to:
   * Local node:
     1. Fill in `Partner` as your remote node ID.
     2. Connect to WS-MCP server (default port should connect automatically).
   * Hosted/VPS node:
     1. Fill in `Partner` as your local node ID.
     2. Click `Accept Clients` (may work by default).

If configured correctly, when you open kibitz on your mobile device, you should be able to access tools just like from your local node!
