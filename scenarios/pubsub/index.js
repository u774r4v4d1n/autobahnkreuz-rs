const {
    AnonymousAuthProvider,
    Connection,
    JSONSerializer,
    NodeWebSocketTransport,
} = require('@verkehrsministerium/kraftfahrstrasse');

const snooze = ms => new Promise(resolve => setTimeout(resolve, ms));
const worker_count = 3;

function randomInt(low, high) {
    return Math.floor(Math.random() * (high - low) + low);
}

async function publish(connection, ordinal) {
    while(ordinal == 0) {
        await snooze(randomInt(200, 1000));
        console.log('publishing new topic from', connection.sessionId);
        connection.Publish('autobahnkreuz.scenarios.pubsub', [ordinal]);
    }
}

async function worker(ordinal) {
    const connection = new Connection({
        endpoint: 'ws://localhost:809' + ordinal + '/',
        realm: 'default',

        serializer: new JSONSerializer(),
        transport: NodeWebSocketTransport,
        authProvider: new AnonymousAuthProvider(),

        logFunction: (level, timestamp, file, msg) => {
            if (level === 'INFO' || level === 'WARNING' || level === 'ERROR') {
                console.log(level, timestamp, file, msg);
            }
        },
    });

    await connection.Open();

    connection.Subscribe(
        'autobahnkreuz.scenarios.pubsub',
        (args, _kwargs, _details) => {
            console.log('received topic from', args[0], 'on', ordinal);
        },
    );

    publish(connection, ordinal);
};

async function main() {
    let workers = [];
    for (let i = 0; i < worker_count; i++) {
        workers.push(worker(i));
    }

    for (let i = 0; i < worker_count; i++) {
        await workers[i];
    }
}

main();
