import { StateManagerServiceClientImpl, Checkpoint } from "./gen/proto/state_manager/state_manager";
import { Client as GrpcClient, requestCallback, credentials } from "@grpc/grpc-js";

export type CheckpointId = string;

export class Client {
  etag!: string;

  private rpc: StateManagerServiceClientImpl;

  constructor(
    readonly grpc: GrpcClient,
    readonly appId: string,
  ) {
    const sendRequest = (service: string, method: string, data: Uint8Array): Promise<Uint8Array> => {
      const path = `/${service}/${method}`
      return new Promise((resolve, reject) => {
        const requestCallback: requestCallback<any> = (err, res) => {
          if (err) {
            reject(err);
          } else {
            resolve(res);
          }
        };

        function passThrough(argument: any) {
          return argument;
        }

        // Using passThrough as the serialize and deserialize functions
        grpc.makeUnaryRequest(path, passThrough, passThrough, data, requestCallback);
      });
    };

    this.rpc = new StateManagerServiceClientImpl({ request: sendRequest });
  }


  async initApp(): Promise<void> {
    const response = await this.rpc.InitApp({ appId: this.appId });
    this.etag = response.etag;
  }

  async get(keys: string[]): Promise<Record<string, Uint8Array>> {
    const response = await this.rpc.Get({ appId: this.appId, keys });
    this.etag = response.etag;
    return Object.fromEntries(response.parts.map(part => [part.key, Uint8Array.from(part.value)]));
  }

  async set(parts: Record<string, Uint8Array>): Promise<void> {
    const pbParts = Object.entries(parts).map(([key, value]) => ({ key, value }));
    const response = await this.rpc.Set({
      appId: this.appId,
      etag: this.etag,
      parts: pbParts,
    });
    this.etag = response.etag;
  }

  async checkpoints(): Promise<Checkpoint[]> {
    const response = await this.rpc.Checkpoints({ appId: this.appId });
    this.etag = response.etag;
    return response.checkpoints;
  }

  async create_checkpoint(payload: string): Promise<CheckpointId> {
    const response = await this.rpc.CreateCheckpoint({
      appId: this.appId, etag: this.etag, payload
    });
    this.etag = response.etag;
    return response.id;
  }

  async revert(id: CheckpointId): Promise<void> {
    const response = await this.rpc.Revert({
      appId: this.appId, etag: this.etag, checkpointId: id
    });
    this.etag = response.etag;
  }

  async cleanup(untilCheckpoint: CheckpointId): Promise<void> {
    const response = await this.rpc.Cleanup({
      appId: this.appId, etag: this.etag, untilCheckpoint
    });
    this.etag = response.etag;
  }
}

export function createNoAuthClient(address: string, appId: string) {
  const grpc = new GrpcClient(address, credentials.createInsecure());
  return new Client(grpc, appId);
}
