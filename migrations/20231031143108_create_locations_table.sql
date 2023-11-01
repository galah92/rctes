CREATE TABLE locations (
    name          TEXT PRIMARY KEY,
    population    BIGINT NOT NULL DEFAULT 0,
    parent        TEXT REFERENCES locations(name)
);

INSERT INTO locations(name, parent, population) VALUES
    ('United States', NULL, 329500000),
    ('Australia', NULL, 25690000),
    ('Spain', NULL, 47350000),
    ('Texas', 'United States', 29000000),
    ('California', 'United States', 39510000),
    ('New South Wales', 'Australia', 8166000),
    ('Victoria', 'Australia', 6681000),
    ('Comunidad de Madrid', 'Spain', 6642000),
    ('Dallas', 'Texas', 1331000),
    ('Austin', 'Texas', 950807),
    ('Houston', 'Texas', 2310000),
    ('Los Angeles', 'California', 3967000),
    ('San Francisco', 'California', 874961),
    ('San Diego', 'California', 1410000),
    ('Sydney', 'New South Wales', 5312000),
    ('Newcastle', 'New South Wales', 322278),
    ('Melbourne', 'Victoria', 5078000),
    ('Geelong', 'Victoria', 253269),
    ('Madrid', 'Comunidad de Madrid', 3223000),
    ('Móstoles', 'Comunidad de Madrid', 207095);
