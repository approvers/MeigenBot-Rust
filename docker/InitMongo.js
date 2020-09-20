let meigenDB = new Mongo().getDB("meigen");
meigenDB.entries.createIndex({ id: 1 }, { unique: true });
meigenDB.entries.insert({ id: 1, author: "かわえもん", content: "てーすと" });
