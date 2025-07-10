import random
import csv
import string
from datetime import datetime, timedelta

random.seed(42)

def random_ipv4():
    return ".".join(str(random.randint(1, 254)) for _ in range(4))

def random_string(length):
    return ''.join(random.choices(string.ascii_letters, k=length))

def random_city():
    # Just make up city names of length 6-10
    return random_string(random.randint(6, 10))

def random_country():
    # Just make up country names of length 7-15
    return random_string(random.randint(7, 15))

def random_datetime_within_years(years_back=3):
    now = datetime.now()
    start = now - timedelta(days=years_back * 365)
    random_dt = start + timedelta(seconds=random.randint(0, int((now - start).total_seconds())))
    return random_dt

unique_lat_longs = set()
while len(unique_lat_longs) < 1_500:
    lat = round(random.uniform(-90, 90), 6)
    lon = round(random.uniform(-180, 180), 6)
    unique_lat_longs.add((lat, lon))
unique_lat_longs = list(unique_lat_longs)

with open("visitation_data.csv", "w", newline="") as f:
    writer = csv.writer(f)
    for i in range(3_000_000):
        lat, lon = random.choice(unique_lat_longs)
        ip = random_ipv4()
        city = random_city()
        country = random_country()
        visited_at = random_datetime_within_years(3)
        writer.writerow([lat, lon, ip, city, country, visited_at.isoformat()])
