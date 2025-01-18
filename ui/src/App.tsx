import { useState, useEffect } from "react";
import KinodeClientApi from "@kinode/client-api";
import "./App.css";
import { ConnectionType } from "./types/FwdWs";
import useFwdWsStore from "./store/fwd_ws";

const BASE_URL = import.meta.env.BASE_URL;
if (window.our) window.our.process = BASE_URL?.replace("/", "");

const PROXY_TARGET = `${(import.meta.env.VITE_NODE_URL || "http://localhost:8080")}${BASE_URL}`;
const WEBSOCKET_URL = import.meta.env.DEV
  ? `${PROXY_TARGET.replace('http', 'ws')}/ws`
  : undefined;

function App() {
  const { state, updateState, setPartner, connectToServer, acceptClients, disconnect, refreshState } = useFwdWsStore();
  const [nodeConnected, setNodeConnected] = useState(true);
  const [api, setApi] = useState<KinodeClientApi | undefined>();

  const [partner, setPartnerInput] = useState("");
  const [wsUrl, setWsUrl] = useState("");

  // Setup WebSocket connections and state refresh
  useEffect(() => {
    refreshState().catch(console.error);

    if (window.our?.node && window.our?.process) {
      const api = new KinodeClientApi({
        uri: WEBSOCKET_URL,
        nodeId: window.our.node,
        processId: window.our.process,
        onOpen: () => {
          console.log("Connected to Kinode");
        },
        onMessage: (json) => {
          try {
            console.log("WebSocket received:", json);
            refreshState().catch(console.error);
          } catch (error) {
            console.error("Error handling WebSocket message:", error);
          }
        },
      });
      setApi(api);
    } else {
      setNodeConnected(false);
    }
  }, []);

  return (
    <div style={{ width: "100%" }}>
      <div style={{ position: "absolute", top: 4, left: 8 }}>
        ID: <strong>{window.our?.node}</strong>
      </div>

      {!nodeConnected && (
        <div className="node-not-connected">
          <h2 style={{ color: "red" }}>Node not connected</h2>
          <h4>
            Start a node at {PROXY_TARGET} first.
          </h4>
        </div>
      )}

      <h2>WebSocket Forwarder</h2>
      <div className="card">
        <div style={{ marginBottom: '2em' }}>
          <h3>Current State</h3>
          <div>Partner: {state.partner || 'None'}</div>
          <div>Connection: {state.connection}</div>
          <div>WebSocket URL: {state.wsUrl || 'None'}</div>
        </div>

        <div style={{ marginBottom: '2em' }}>
          <h3>Set Partner</h3>
          <div className="input-row">
            <input
              type="text"
              value={partner}
              onChange={(e) => setPartnerInput(e.target.value)}
              placeholder="Partner node ID"
            />
            <button onClick={() => setPartner(partner || null)}>
              {partner ? 'Set Partner' : 'Clear Partner'}
            </button>
          </div>
        </div>

        <div style={{ marginBottom: '2em' }}>
          <h3>WebSocket Connection</h3>
          {state.connection === ConnectionType.None ? (
            <>
              <div className="input-row">
                <input
                  type="text"
                  value={wsUrl}
                  onChange={(e) => setWsUrl(e.target.value)}
                  placeholder="WebSocket URL or endpoint"
                />
                <button onClick={() => connectToServer(wsUrl)}>
                  Connect to Server
                </button>
                <button onClick={() => acceptClients(wsUrl || "/ws")}>
                  Accept Clients
                </button>
              </div>
            </>
          ) : (
            <button onClick={() => disconnect()}>Disconnect</button>
          )}
        </div>
      </div>
    </div>
  );
}

export default App;
