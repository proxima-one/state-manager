import { strict as assert } from "assert";
import { StateManagerServiceClientImpl, Checkpoint } from "./gen/proto/state_manager/state_manager";
import { Client as GrpcClient, requestCallback, credentials } from "@grpc/grpc-js";
import { sleep } from "@proxima-one/proxima-utils";

export type CheckpointId = string;

export class Client {
  private etag: string | undefined;

  private rpc: StateManagerServiceClientImpl;

  constructor(
    readonly grpc: GrpcClient,
    readonly appId: string,
  ) {
    const sendRequest = (path: string, data: Uint8Array): Promise<Uint8Array> => {
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

    const sendRequestWithReties = async (service: string, method: string, data: Uint8Array): Promise<Uint8Array> => {
      const RETRIES = 5;
      const DELAY = 200;

      const path = `/${service}/${method}`
      for (let i = 0; i < RETRIES; ++i) {
        try {
          return await sendRequest(path, data);
        } catch (e) {
          console.error(`Error during gRPC call, retrying. ${e}`);
        }
        await sleep(DELAY);
      }
      return sendRequest(path, data);
    }

    this.rpc = new StateManagerServiceClientImpl({ request: sendRequestWithReties });
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
    assert(this.etag);
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
    assert(this.etag);
    const response = await this.rpc.CreateCheckpoint({
      appId: this.appId, etag: this.etag, payload
    });
    this.etag = response.etag;
    return response.id;
  }

  async revert(id: CheckpointId): Promise<void> {
    assert(this.etag);
    const response = await this.rpc.Revert({
      appId: this.appId, etag: this.etag, checkpointId: id
    });
    this.etag = response.etag;
  }

  async cleanup(untilCheckpoint: CheckpointId): Promise<void> {
    assert(this.etag);
    const response = await this.rpc.Cleanup({
      appId: this.appId, etag: this.etag, untilCheckpoint
    });
    this.etag = response.etag;
  }

  async reset(): Promise<void> {
    assert(this.etag);
    const response = await this.rpc.Reset({
      appId: this.appId, etag: this.etag
    });
    this.etag = response.etag;
  }
}

export function createNoAuthClient(address: string, appId: string) {
  const MB = 2 ** 20;
  const creds = address.endsWith("443") ? credentials.createSsl() : credentials.createInsecure();
  const grpc = new GrpcClient(address, creds, {
    "grpc.max_receive_message_length": 128 * MB,
    "grpc.max_send_message_length": 32 * MB,
  });
  return new Client(grpc, appId);
}
