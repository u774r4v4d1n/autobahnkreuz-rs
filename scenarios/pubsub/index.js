const { Connection } = require('autobahn');

const snooze = ms => new Promise(resolve => setTimeout(resolve, ms));
const worker_count = 2;

function randomInt(low, high) {
    return Math.floor(Math.random() * (high - low) + low);
}

async function publish(session, ordinal) {
    while(true) {
        await snooze(randomInt(1000, 5000));
        console.log('publishing new topic from', ordinal);
        session.publish('autobahnkreuz.scenarios.pubsub', [ordinal]);
    }
}

let received = new Array(worker_count).fill(1);

async function worker(ordinal) {
    const connection = new Connection({
        url: 'ws://localhost:809' + ordinal + '/',
        realm: 'default',
    });

    connection.onopen = (session) => {
        session.subscribe(
            'autobahnkreuz.scenarios.pubsub',
            (args, _kwargs, _details) => {
                received[args[0]] += 1;

                console.log('received topic from', args[0]);
                /*if (received[args[0]] >= worker_count) {
                    console.log('received', worker_count, 'topics from', args[0]);
                    received[args[0]] = 0;
                }*/
            },
        );

        publish(session, ordinal);
    };

    connection.open();
}

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
