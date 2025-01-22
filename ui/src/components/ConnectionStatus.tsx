import { ConnectionType } from "../types/FwdWs";

interface ConnectionStatusProps {
  connectionType: ConnectionType;
}

export function ConnectionStatus({ connectionType }: ConnectionStatusProps) {
  const isConnected = connectionType !== ConnectionType.None;
  
  return (
    <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
      <div
        style={{
          width: '12px',
          height: '12px',
          borderRadius: '50%',
          backgroundColor: isConnected ? '#4CAF50' : '#f44336',
          transition: 'background-color 0.3s'
        }}
      />
      <span>{isConnected ? 'Connected' : 'Disconnected'}</span>
    </div>
  );
}