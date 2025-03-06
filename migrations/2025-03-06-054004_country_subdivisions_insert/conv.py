import csv
import sys
import argparse

def process_file(input_filename, output_filename):
    rows = []  # list to store the formatted value strings for each row
    seen = set()  # set to store (country_alpha2, regional_code) combinations to de-duplicate

    with open(input_filename, newline='', encoding='utf-8') as csvfile:
        # CSV uses ";" as delimiter
        reader = csv.reader(csvfile, delimiter=';')
        header = next(reader)
        # Check whether the first row is truly a header; if not, process from beginning.
        if header[0].strip().upper() != "COUNTRY SHORT CODE":
            csvfile.seek(0)
            reader = csv.reader(csvfile, delimiter=';')
        for line in reader:
            if len(line) < 5:
                continue  # Skip invalid/malformed rows

            # Read and clean fields from CSV
            country_alpha2 = line[0].strip().strip('"')
            region_name = line[1].strip().strip('"')
            # Capitalize the region_type so that 'province' becomes 'Province', etc.
            region_type = line[2].strip().strip('"').capitalize()
            regional_code = line[3].strip().strip('"')
            regional_number_code = line[4].strip()  # used as subdivision_id

            # Replace country 'KP' with 'KR'
            if country_alpha2.upper() == "KP":
                country_alpha2 = "KR"

            # Check for duplicate: same country and regional code already processed.
            dedup_key = (country_alpha2, regional_code)
            if dedup_key in seen:
                continue  # Skip duplicate row
            seen.add(dedup_key)

            try:
                subdivision_id = int(regional_number_code)
            except ValueError:
                sys.exit(f"Invalid subdivision id: {regional_number_code}")

            # Build a subquery for the country_code using the two-letter code.
            country_code_expr = f"(SELECT country_code FROM public.iso_country WHERE country_alpha2 = '{country_alpha2}')"

            # Escape any single quotes in text fields (for SQL query safety)
            subdivision_code_sql = regional_code.replace("'", "''")
            subdivision_name_sql = region_name.replace("'", "''")
            subdivision_type_sql = region_type.replace("'", "''")

            # Build a VALUES tuple using the subquery expression for country_code.
            row_str = f"({subdivision_id}, {country_code_expr}, '{subdivision_code_sql}', '{subdivision_name_sql}', '{subdivision_type_sql}')"
            rows.append(row_str)

    # Generate the full SQL INSERT statement as a batch insert.
    with open(output_filename, 'w', encoding='utf-8') as outfile:
        outfile.write("INSERT INTO public.iso_country_subdivision (subdivision_id, country_code, subdivision_code, subdivision_name, subdivision_type) VALUES\n")
        outfile.write(",\n".join(rows))
        outfile.write(";\n")

    print(f"SQL batch insert statement written to {output_filename}")

def main():
    parser = argparse.ArgumentParser(description="Generate PostgreSQL batch insert SQL for iso_country_subdivision.")
    parser.add_argument("inputfile", help="Input CSV filename")
    parser.add_argument("outputfile", help="Output SQL filename")
    args = parser.parse_args()
    process_file(args.inputfile, args.outputfile)

if __name__ == '__main__':
    main()
