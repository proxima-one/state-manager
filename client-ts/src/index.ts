import { credentials, ServiceError } from "@grpc/grpc-js";

import { StateManagerServiceClient } from "./gen/proto/state_manager/state_manager_grpc_pb";
import * as pb from "./gen/proto/state_manager/state_manager_pb"

class Client {
  etag!: string;

  constructor(
    readonly grpc: StateManagerServiceClient,
    readonly appId: string,
  ) { }

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

  async cleanup(untilCheckpoint: string): Promise<void> {
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

async function run(args: { port: number }) {
  const grpcClient = new StateManagerServiceClient(`localhost:${args.port}`, credentials.createInsecure());
  const client = new Client(grpcClient, "test-app");

  try {
    const checkpoints = await client.checkpoints();
    console.log(`Got ${checkpoints.length} checkpoints`);
    await client.cleanup(checkpoints[0].id);
  } catch (e) {
    console.error((e as ServiceError).message);
  }
}

run({ port: 50051 });
