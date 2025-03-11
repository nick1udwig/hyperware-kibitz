import { useState, useEffect } from "react";
import HyperwareClientApi from "@hyperware-ai/client-api";
import "./App.css";
import { ConnectionType } from "./types/FwdWs";
import useFwdWsStore from "./store/fwd_ws";
import { ConnectionStatus } from "./components/ConnectionStatus";

const BASE_URL = import.meta.env.BASE_URL;
if (window.our) window.our.process = BASE_URL?.replace("/", "");

const PROXY_TARGET = `${(import.meta.env.VITE_NODE_URL || "http://localhost:8080")}${BASE_URL}`;
const WEBSOCKET_URL = import.meta.env.DEV
  ? `${PROXY_TARGET.replace('http', 'ws')}/ws`
  : undefined;

function App() {
  const { state, setPartner, connectToServer, acceptClients, disconnect, refreshState } = useFwdWsStore();
  const [nodeConnected, setNodeConnected] = useState(true);

  const [partner, setPartnerInput] = useState(state.partner || "");
  const [wsUrl, setWsUrl] = useState(state.wsUrl || "");

  // Update input fields when state changes
  useEffect(() => {
    setPartnerInput(state.partner || "");
    setWsUrl(state.wsUrl || "ws://localhost:10125");
  }, [state.partner, state.wsUrl]);

  // Setup WebSocket connections and state refresh
  useEffect(() => {
    // Initial state fetch
    refreshState().catch(console.error);

    // Set up periodic polling
    const pollInterval = setInterval(() => {
      refreshState().catch(console.error);
    }, 2000); // Poll every 2 seconds

    if (window.our?.node && window.our?.process) {
      new HyperwareClientApi({
        uri: WEBSOCKET_URL,
        nodeId: window.our.node,
        processId: window.our.process,
        onOpen: () => {
          console.log("Connected to Kinode");
          refreshState().catch(console.error);
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
    } else {
      setNodeConnected(false);
    }

    // Cleanup polling on unmount
    return () => clearInterval(pollInterval);
  }, [refreshState]);

  return (
    <div style={{ width: "100%" }}>
      <div style={{ position: "absolute", top: 4, left: 8, display: "flex", alignItems: "center", gap: "16px" }}>
        <div>ID: <strong>{window.our?.node}</strong></div>
        <ConnectionStatus connectionType={state.connection} />
      </div>
      <div style={{ position: "absolute", top: 20, left: 8 }}>
        <a href={`${window.location.protocol}//${window.location.host}/kibitz:kibitz:nick.hypr`}>Kibitz</a>
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
