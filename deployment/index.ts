import * as pulumi from "@pulumi/pulumi";
import * as k8s from "@pulumi/kubernetes";
import { strict as assert } from "assert";

const cfg = new pulumi.Config();  // Getting config from Pulumi.{stack}.yaml file

const proximaNode = process.env["PROXIMA_NODE"];
console.log("Proxima Node: ", proximaNode)

const imageName = process.env["IMAGE_NAME"];
console.log("Image name:", imageName)
assert(imageName);

const version = process.env["VERSION"];
console.log("Version:", version)
assert(version);
const versionParts = version.split(".");
assert(versionParts.length > 2);

const infraStack = new pulumi.StackReference(`proxima-one/proxima-gke/${proximaNode}`, {});
const kubeconfig = infraStack.getOutput("kubeconfig");
const k8sProvider = new k8s.Provider("infra-k8s", {
  kubeconfig: kubeconfig,
});

const servicesStack = new pulumi.StackReference(`proxima-one/${cfg.require<string>("services-stack-name")}-services/default`, {});
const webAppsOptions = servicesStack.requireOutput("webServices") as pulumi.Output<{ namespace: string, imagePullSecret: string }>;

const serverGrpcPort = cfg.require<string>("server-grpc-port");
const dbPath = cfg.require<string>("db-path");
const serviceName = "state-manager";
const server_labels: Record<string, string> = {
  app: serviceName,
};

const volumeName = "state-manager-db";

const pvc = new k8s.core.v1.PersistentVolumeClaim(volumeName, {
  metadata: {
    namespace: webAppsOptions.namespace,
  },
  spec: {
    storageClassName: "standard",
    accessModes: ["ReadWriteOnce"],
    resources: {
      requests: {
        storage: "128G"
      }
    }
  }
}, { provider: k8sProvider });

const server = new k8s.apps.v1.Deployment(serviceName, {
  metadata: {
    namespace: webAppsOptions.namespace,
  },
  spec: {
    replicas: 1,
    selector: {
      matchLabels: server_labels
    },
    template: {
      metadata: {
        labels: server_labels,
      },
      spec: {
        restartPolicy: "Always",
        imagePullSecrets: [{
          name: webAppsOptions.imagePullSecret
        }],
        containers: [{
          image: imageName,
          name: "state-manager",
          args: [
          ],
          env: [
            {
              name: "PORT",
              value: serverGrpcPort
            },
            {
              name: "DB_PATH",
              value: dbPath
            }
          ],
          ports: [
            {
              containerPort: parseInt(serverGrpcPort),
            }
          ],
          volumeMounts: [
            {
              name: volumeName,
              mountPath: dbPath
            }
          ],
          resources: {
            requests: {
              memory: "1000Mi",
              cpu: "50m",
            },
            limits: {
              memory: "4000Mi",
              cpu: "1000m",
            }
          }
        }],
        volumes: [{
          name: volumeName,
          persistentVolumeClaim: {
            claimName: pvc.metadata.name,
            readOnly: false,
          }
        }],
      }
    },
  }
}, { provider: k8sProvider });

const service = new k8s.core.v1.Service(serviceName, {
  metadata: {
    namespace: webAppsOptions.namespace,
  },
  spec: {
    selector: server_labels,
    ports: [
      {
        name: "grpc",
        protocol: "TCP",
        port: parseInt(serverGrpcPort),
        targetPort: parseInt(serverGrpcPort)
      }
    ],
  }
}, { dependsOn: server, provider: k8sProvider });
