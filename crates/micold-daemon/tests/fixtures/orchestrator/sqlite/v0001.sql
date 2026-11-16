-- Schema version 1 seed (#1948, FR-013): one row in every table, for the migration chain test.
-- Never edited once shipped: the chain test upgrades a database seeded with it to the newest schema.
INSERT INTO daemons VALUES ('default','d1','host','alpha',81,'["fs"]','online',100,'u1');
INSERT INTO projects VALUES ('default','p1','d1','proj','/srv/proj','u1',101);
INSERT INTO sessions VALUES ('default','s1','d1','p1','u1',102);
INSERT INTO users VALUES ('default','u1','Ada','owner',103,NULL);
INSERT INTO identities VALUES ('default','github','42','u1','ada@example.org',1);
INSERT INTO invites VALUES ('default','i1','ih','viewer','bob@example.org','u1',104,204,NULL,NULL);
INSERT INTO bootstrap VALUES ('default','ch',105,106);
INSERT INTO refresh_tokens VALUES ('default','rh','u1','f1',107,207,NULL);
INSERT INTO cert_serials VALUES ('default','0a','d1',108,208);
INSERT INTO cert_revocations VALUES ('default','0b',109,'superseded');
INSERT INTO enrollments VALUES ('default','eh','u1',110,210,'d1',111);
INSERT INTO audit_log (tenant_id, at, actor, action, target, detail) VALUES ('default',112,'u1','login','u1','{}');
INSERT INTO imports VALUES ('default','registry.json','sha256:00',113);
