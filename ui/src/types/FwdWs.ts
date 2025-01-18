export enum ConnectionType {
  None = "None",
  ToWsServer = "ToWsServer",
  ToWsClient = "ToWsClient",
}

export interface ProcessState {
  partner: string | null
  connection: ConnectionType
  wsUrl: string | null
}

export type SetPartnerRequest = {
  SetPartner: string | null
}

export type ConnectToServerRequest = {
  ConnectToServer: string
}

export type AcceptClientsRequest = {
  AcceptClients: string
}

export type DisconnectRequest = {
  Disconnect: null
}

export type GetStateRequest = {
  GetState: null
}

export type ForwardRequest = {
  Forward: string
}

export type FwdWsRequest =
  | SetPartnerRequest
  | ConnectToServerRequest
  | AcceptClientsRequest
  | DisconnectRequest
  | GetStateRequest
  | ForwardRequest
