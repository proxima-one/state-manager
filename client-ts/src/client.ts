import { StateManagerServiceClient } from "./gen/proto/state_manager/state_manager_grpc_pb";
import * as pb from "./gen/proto/state_manager/state_manager_pb"

export type CheckpointId = string;

export class Client {
  etag!: string;

  constructor(
    readonly grpc: StateManagerServiceClient,
    readonly appId: string,
  ) { }


  async initApp(): Promise<void> {
    const request = new pb.InitAppRequest().setAppId(this.appId);
    return new Promise((resolve, reject) =>
      this.grpc.initApp(request, (error, response) => {
        if (error) {
          reject(error);
        } else {
          this.etag = response.getEtag()
          resolve();
        }
      })
    );
  }

  async get(keys: string[]): Promise<Record<string, string | Uint8Array>> {
    const request = new pb.GetRequest().setAppId(this.appId).setKeysList(keys);
    return new Promise((resolve, reject) =>
      this.grpc.get(request, (error, response) => {
        if (error) {
          reject(error);
        } else {
          this.etag = response.getEtag()
          resolve(Object.fromEntries(response.getPartsList().map(part => [part.getKey(), part.getValue()])));
        }
      })
    );
  }

  async set(parts: Record<string, Uint8Array>): Promise<void> {
    let pbParts = Object.entries(parts).map(([key, value]) => (new pb.Part().setKey(key).setValue(value)));
    const request = new pb.SetRequest()
      .setAppId(this.appId).setEtag(this.etag)
      .setPartsList(pbParts);
    return new Promise((resolve, reject) =>
      this.grpc.set(request, (error, response) => {
        if (error) {
          reject(error);
        } else {
          this.etag = response.getEtag()
          resolve();
        }
      })
    );
  }

  async checkpoints(): Promise<pb.Checkpoint.AsObject[]> {
    const request = new pb.CheckpointsRequest().setAppId(this.appId);
    return new Promise((resolve, reject) =>
      this.grpc.checkpoints(request, (error, response) => {
        if (error) {
          reject(error);
        } else {
          this.etag = response.getEtag()
          resolve(response.toObject().checkpointsList);
        }
      })
    );
  }

  async create_checkpoint(payload: string): Promise<CheckpointId> {
    const request = new pb.CreateCheckpointRequest()
      .setAppId(this.appId).setEtag(this.etag)
      .setPayload(payload);
    return new Promise((resolve, reject) =>
      this.grpc.createCheckpoint(request, (error, response) => {
        if (error) {
          reject(error);
        } else {
          this.etag = response.getEtag()
          resolve(response.getId());
        }
      })
    );
  }

  async revert(id: CheckpointId): Promise<void> {
    const request = new pb.RevertRequest()
      .setAppId(this.appId).setEtag(this.etag)
      .setCheckpointId(id);
    return new Promise((resolve, reject) =>
      this.grpc.revert(request, (error, response) => {
        if (error) {
          reject(error);
        } else {
          this.etag = response.getEtag()
          resolve();
        }
      })
    );
  }

    async cleanup(untilCheckpoint: CheckpointId): Promise<void> {
    const request = new pb.CleanupRequest()
      .setAppId(this.appId)
      .setEtag(this.etag)
      .setUntilCheckpoint(untilCheckpoint);
    return new Promise((resolve, reject) =>
      this.grpc.cleanup(request, (error, response) => {
        if (error) {
          reject(error);
        } else {
          this.etag = response.getEtag()
          resolve();
        }
      })
    );
  }
}
