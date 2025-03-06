ALTER TABLE public.iso_country
ADD COLUMN country_flag bpchar (2),
ADD COLUMN is_country boolean NOT NULL DEFAULT true;

CREATE INDEX idx_iso_country_is_country ON public.iso_country USING btree (is_country);

UPDATE public.iso_country
SET
    country_flag = CASE country_code
        WHEN 4 THEN '🇦🇫' -- Afghanistan
        WHEN 248 THEN '🇦🇽' -- Åland Islands
        WHEN 8 THEN '🇦🇱' -- Albania
        WHEN 12 THEN '🇩🇿' -- Algeria
        WHEN 16 THEN '🇦🇸' -- American Samoa
        WHEN 20 THEN '🇦🇩' -- Andorra
        WHEN 24 THEN '🇦🇴' -- Angola
        WHEN 660 THEN '🇦🇮' -- Anguilla
        WHEN 28 THEN '🇦🇬' -- Antigua and Barbuda
        WHEN 32 THEN '🇦🇷' -- Argentina
        WHEN 51 THEN '🇦🇲' -- Armenia
        WHEN 533 THEN '🇦🇼' -- Aruba
        WHEN 36 THEN '🇦🇺' -- Australia
        WHEN 40 THEN '🇦🇹' -- Austria
        WHEN 31 THEN '🇦🇿' -- Azerbaijan
        WHEN 44 THEN '🇧🇸' -- Bahamas
        WHEN 48 THEN '🇧🇭' -- Bahrain
        WHEN 50 THEN '🇧🇩' -- Bangladesh
        WHEN 52 THEN '🇧🇧' -- Barbados
        WHEN 112 THEN '🇧🇾' -- Belarus
        WHEN 56 THEN '🇧🇪' -- Belgium
        WHEN 84 THEN '🇧🇿' -- Belize
        WHEN 204 THEN '🇧🇯' -- Benin
        WHEN 60 THEN '🇧🇲' -- Bermuda
        WHEN 64 THEN '🇧🇹' -- Bhutan
        WHEN 68 THEN '🇧🇴' -- Bolivia, Plurinational State of
        WHEN 535 THEN '🇧🇶' -- Bonaire, Sint Eustatius and Saba
        WHEN 70 THEN '🇧🇦' -- Bosnia and Herzegovina
        WHEN 72 THEN '🇧🇼' -- Botswana
        WHEN 74 THEN '🇧🇻' -- Bouvet Island
        WHEN 76 THEN '🇧🇷' -- Brazil
        WHEN 86 THEN '🇮🇴' -- British Indian Ocean Territory
        WHEN 96 THEN '🇧🇳' -- Brunei Darussalam
        WHEN 100 THEN '🇧🇬' -- Bulgaria
        WHEN 854 THEN '🇧🇫' -- Burkina Faso
        WHEN 108 THEN '🇧🇮' -- Burundi
        WHEN 132 THEN '🇨🇻' -- Cabo Verde
        WHEN 116 THEN '🇰🇭' -- Cambodia
        WHEN 120 THEN '🇨🇲' -- Cameroon
        WHEN 124 THEN '🇨🇦' -- Canada
        WHEN 136 THEN '🇰🇾' -- Cayman Islands
        WHEN 140 THEN '🇨🇫' -- Central African Republic
        WHEN 148 THEN '🇹🇩' -- Chad
        WHEN 152 THEN '🇨🇱' -- Chile
        WHEN 156 THEN '🇨🇳' -- China
        WHEN 162 THEN '🇨🇽' -- Christmas Island
        WHEN 166 THEN '🇨🇨' -- Cocos (Keeling) Islands
        WHEN 170 THEN '🇨🇴' -- Colombia
        WHEN 174 THEN '🇰🇲' -- Comoros
        WHEN 178 THEN '🇨🇬' -- Congo
        WHEN 180 THEN '🇨🇩' -- Congo, Democratic Republic of the
        WHEN 184 THEN '🇨🇰' -- Cook Islands
        WHEN 188 THEN '🇨🇷' -- Costa Rica
        WHEN 384 THEN '🇨🇮' -- Côte d'Ivoire
        WHEN 191 THEN '🇭🇷' -- Croatia
        WHEN 192 THEN '🇨🇺' -- Cuba
        WHEN 531 THEN '🇨🇼' -- Curaçao
        WHEN 196 THEN '🇨🇾' -- Cyprus
        WHEN 203 THEN '🇨🇿' -- Czechia
        WHEN 208 THEN '🇩🇰' -- Denmark
        WHEN 262 THEN '🇩🇯' -- Djibouti
        WHEN 212 THEN '🇩🇲' -- Dominica
        WHEN 214 THEN '🇩🇴' -- Dominican Republic
        WHEN 218 THEN '🇪🇨' -- Ecuador
        WHEN 818 THEN '🇪🇬' -- Egypt
        WHEN 222 THEN '🇸🇻' -- El Salvador
        WHEN 226 THEN '🇬🇶' -- Equatorial Guinea
        WHEN 232 THEN '🇪🇷' -- Eritrea
        WHEN 233 THEN '🇪🇪' -- Estonia
        WHEN 748 THEN '🇸🇿' -- Eswatini
        WHEN 231 THEN '🇪🇹' -- Ethiopia
        WHEN 238 THEN '🇫🇰' -- Falkland Islands (Malvinas)
        WHEN 234 THEN '🇫🇴' -- Faroe Islands
        WHEN 242 THEN '🇫🇯' -- Fiji
        WHEN 246 THEN '🇫🇮' -- Finland
        WHEN 250 THEN '🇫🇷' -- France
        WHEN 254 THEN '🇬🇫' -- French Guiana
        WHEN 258 THEN '🇵🇫' -- French Polynesia
        WHEN 260 THEN '🇹🇫' -- French Southern Territories
        WHEN 266 THEN '🇬🇦' -- Gabon
        WHEN 270 THEN '🇬🇲' -- Gambia
        WHEN 268 THEN '🇬🇪' -- Georgia
        WHEN 276 THEN '🇩🇪' -- Germany
        WHEN 288 THEN '🇬🇭' -- Ghana
        WHEN 292 THEN '🇬🇮' -- Gibraltar
        WHEN 300 THEN '🇬🇷' -- Greece
        WHEN 304 THEN '🇬🇱' -- Greenland
        WHEN 308 THEN '🇬🇩' -- Grenada
        WHEN 312 THEN '🇬🇵' -- Guadeloupe
        WHEN 316 THEN '🇬🇺' -- Guam
        WHEN 320 THEN '🇬🇹' -- Guatemala
        WHEN 831 THEN '🇬🇬' -- Guernsey
        WHEN 324 THEN '🇬🇳' -- Guinea
        WHEN 624 THEN '🇬🇼' -- Guinea-Bissau
        WHEN 328 THEN '🇬🇾' -- Guyana
        WHEN 332 THEN '🇭🇹' -- Haiti
        WHEN 336 THEN '🇻🇦' -- Holy See
        WHEN 340 THEN '🇭🇳' -- Honduras
        WHEN 344 THEN '🇭🇰' -- Hong Kong
        WHEN 348 THEN '🇭🇺' -- Hungary
        WHEN 352 THEN '🇮🇸' -- Iceland
        WHEN 356 THEN '🇮🇳' -- India
        WHEN 360 THEN '🇮🇩' -- Indonesia
        WHEN 364 THEN '🇮🇷' -- Iran, Islamic Republic of
        WHEN 368 THEN '🇮🇶' -- Iraq
        WHEN 372 THEN '🇮🇪' -- Ireland
        WHEN 833 THEN '🇮🇲' -- Isle of Man
        WHEN 376 THEN '🇮🇱' -- Israel
        WHEN 380 THEN '🇮🇹' -- Italy
        WHEN 388 THEN '🇯🇲' -- Jamaica
        WHEN 392 THEN '🇯🇵' -- Japan
        WHEN 832 THEN '🇯🇪' -- Jersey
        WHEN 400 THEN '🇯🇴' -- Jordan
        WHEN 398 THEN '🇰🇿' -- Kazakhstan
        WHEN 404 THEN '🇰🇪' -- Kenya
        WHEN 296 THEN '🇰🇮' -- Kiribati
        WHEN 410 THEN '🇰🇷' -- Korea, Republic of
        WHEN 414 THEN '🇰🇼' -- Kuwait
        WHEN 417 THEN '🇰🇬' -- Kyrgyzstan
        WHEN 418 THEN '🇱🇦' -- Lao People's Democratic Republic
        WHEN 428 THEN '🇱🇻' -- Latvia
        WHEN 422 THEN '🇱🇧' -- Lebanon
        WHEN 426 THEN '🇱🇸' -- Lesotho
        WHEN 430 THEN '🇱🇷' -- Liberia
        WHEN 434 THEN '🇱🇾' -- Libya
        WHEN 438 THEN '🇱🇮' -- Liechtenstein
        WHEN 440 THEN '🇱🇹' -- Lithuania
        WHEN 442 THEN '🇱🇺' -- Luxembourg
        WHEN 446 THEN '🇲🇴' -- Macao
        WHEN 450 THEN '🇲🇬' -- Madagascar
        WHEN 454 THEN '🇲🇼' -- Malawi
        WHEN 458 THEN '🇲🇾' -- Malaysia
        WHEN 462 THEN '🇲🇻' -- Maldives
        WHEN 466 THEN '🇲🇱' -- Mali
        WHEN 470 THEN '🇲🇹' -- Malta
        WHEN 584 THEN '🇲🇭' -- Marshall Islands
        WHEN 474 THEN '🇲🇶' -- Martinique
        WHEN 478 THEN '🇲🇷' -- Mauritania
        WHEN 480 THEN '🇲🇺' -- Mauritius
        WHEN 175 THEN '🇾🇹' -- Mayotte
        WHEN 484 THEN '🇲🇽' -- Mexico
        WHEN 583 THEN '🇫🇲' -- Micronesia, Federated States of
        WHEN 498 THEN '🇲🇩' -- Moldova, Republic of
        WHEN 492 THEN '🇲🇨' -- Monaco
        WHEN 496 THEN '🇲🇳' -- Mongolia
        WHEN 499 THEN '🇲🇪' -- Montenegro
        WHEN 500 THEN '🇲🇸' -- Montserrat
        WHEN 504 THEN '🇲🇦' -- Morocco
        WHEN 508 THEN '🇲🇿' -- Mozambique
        WHEN 104 THEN '🇲🇲' -- Myanmar
        WHEN 516 THEN '🇳🇦' -- Namibia
        WHEN 520 THEN '🇳🇷' -- Nauru
        WHEN 524 THEN '🇳🇵' -- Nepal
        WHEN 528 THEN '🇳🇱' -- Netherlands, Kingdom of the
        WHEN 540 THEN '🇳🇨' -- New Caledonia
        WHEN 554 THEN '🇳🇿' -- New Zealand
        WHEN 558 THEN '🇳🇮' -- Nicaragua
        WHEN 562 THEN '🇳🇪' -- Niger
        WHEN 566 THEN '🇳🇬' -- Nigeria
        WHEN 570 THEN '🇳🇺' -- Niue
        WHEN 574 THEN '🇳🇫' -- Norfolk Island
        WHEN 807 THEN '🇲🇰' -- North Macedonia
        WHEN 580 THEN '🇲🇵' -- Northern Mariana Islands
        WHEN 578 THEN '🇳🇴' -- Norway
        WHEN 512 THEN '🇴🇲' -- Oman
        WHEN 586 THEN '🇵🇰' -- Pakistan
        WHEN 585 THEN '🇵🇼' -- Palau
        WHEN 275 THEN '🇵🇸' -- Palestine, State of
        WHEN 591 THEN '🇵🇦' -- Panama
        WHEN 598 THEN '🇵🇬' -- Papua New Guinea
        WHEN 600 THEN '🇵🇾' -- Paraguay
        WHEN 604 THEN '🇵🇪' -- Peru
        WHEN 608 THEN '🇵🇭' -- Philippines
        WHEN 612 THEN '🇵🇳' -- Pitcairn
        WHEN 616 THEN '🇵🇱' -- Poland
        WHEN 620 THEN '🇵🇹' -- Portugal
        WHEN 630 THEN '🇵🇷' -- Puerto Rico
        WHEN 634 THEN '🇶🇦' -- Qatar
        WHEN 638 THEN '🇷🇪' -- Réunion
        WHEN 642 THEN '🇷🇴' -- Romania
        WHEN 643 THEN '🇷🇺' -- Russian Federation
        WHEN 646 THEN '🇷🇼' -- Rwanda
        WHEN 652 THEN '🇧🇱' -- Saint Barthélemy
        WHEN 654 THEN '🇸🇭' -- Saint Helena, Ascension and Tristan da Cunha
        WHEN 659 THEN '🇰🇳' -- Saint Kitts and Nevis
        WHEN 662 THEN '🇱🇨' -- Saint Lucia
        WHEN 663 THEN '🇲🇫' -- Saint Martin (French part)
        WHEN 666 THEN '🇵🇲' -- Saint Pierre and Miquelon
        WHEN 670 THEN '🇻🇨' -- Saint Vincent and the Grenadines
        WHEN 882 THEN '🇼🇸' -- Samoa
        WHEN 674 THEN '🇸🇲' -- San Marino
        WHEN 678 THEN '🇸🇹' -- Sao Tome and Principe
        WHEN 682 THEN '🇸🇦' -- Saudi Arabia
        WHEN 686 THEN '🇸🇳' -- Senegal
        WHEN 688 THEN '🇷🇸' -- Serbia
        WHEN 690 THEN '🇸🇨' -- Seychelles
        WHEN 694 THEN '🇸🇱' -- Sierra Leone
        WHEN 702 THEN '🇸🇬' -- Singapore
        WHEN 534 THEN '🇸🇽' -- Sint Maarten (Dutch part)
        WHEN 703 THEN '🇸🇰' -- Slovakia
        WHEN 705 THEN '🇸🇮' -- Slovenia
        WHEN 90 THEN '🇸🇧' -- Solomon Islands
        WHEN 706 THEN '🇸🇴' -- Somalia
        WHEN 710 THEN '🇿🇦' -- South Africa
        WHEN 239 THEN '🇬🇸' -- South Georgia and the South Sandwich Islands
        WHEN 728 THEN '🇸🇸' -- South Sudan
        WHEN 724 THEN '🇪🇸' -- Spain
        WHEN 144 THEN '🇱🇰' -- Sri Lanka
        WHEN 729 THEN '🇸🇩' -- Sudan
        WHEN 740 THEN '🇸🇷' -- Suriname
        WHEN 744 THEN '🇳🇴' -- Svalbard and Jan Mayen (using Norway’s flag)
        WHEN 752 THEN '🇸🇪' -- Sweden
        WHEN 756 THEN '🇨🇭' -- Switzerland
        WHEN 760 THEN '🇸🇾' -- Syrian Arab Republic
        WHEN 158 THEN '🇹🇼' -- Taiwan, Province of China
        WHEN 762 THEN '🇹🇯' -- Tajikistan
        WHEN 834 THEN '🇹🇿' -- Tanzania, United Republic of
        WHEN 764 THEN '🇹🇭' -- Thailand
        WHEN 626 THEN '🇹🇱' -- Timor-Leste
        WHEN 768 THEN '🇹🇬' -- Togo
        WHEN 772 THEN '🇹🇰' -- Tokelau
        WHEN 776 THEN '🇹🇴' -- Tonga
        WHEN 780 THEN '🇹🇹' -- Trinidad and Tobago
        WHEN 788 THEN '🇹🇳' -- Tunisia
        WHEN 792 THEN '🇹🇷' -- Türkiye
        WHEN 795 THEN '🇹🇲' -- Turkmenistan
        WHEN 796 THEN '🇹🇨' -- Turks and Caicos Islands
        WHEN 798 THEN '🇹🇻' -- Tuvalu
        WHEN 800 THEN '🇺🇬' -- Uganda
        WHEN 804 THEN '🇺🇦' -- Ukraine
        WHEN 784 THEN '🇦🇪' -- United Arab Emirates
        WHEN 826 THEN '🇬🇧' -- United Kingdom of Great Britain and Northern Ireland
        WHEN 840 THEN '🇺🇸' -- United States of America
        WHEN 581 THEN '🇺🇲' -- United States Minor Outlying Islands
        WHEN 858 THEN '🇺🇾' -- Uruguay
        WHEN 860 THEN '🇺🇿' -- Uzbekistan
        WHEN 548 THEN '🇻🇺' -- Vanuatu
        WHEN 862 THEN '🇻🇪' -- Venezuela, Bolivarian Republic of
        WHEN 704 THEN '🇻🇳' -- Viet Nam
        WHEN 92 THEN '🇻🇬' -- Virgin Islands (British)
        WHEN 850 THEN '🇻🇮' -- Virgin Islands (U.S.)
        WHEN 876 THEN '🇼🇫' -- Wallis and Futuna
        WHEN 732 THEN '🇪🇭' -- Western Sahara
        WHEN 887 THEN '🇾🇪' -- Yemen
        WHEN 894 THEN '🇿🇲' -- Zambia
        WHEN 716 THEN '🇿🇼' -- Zimbabwe
        WHEN 334 THEN '🇦🇺' -- Heard Island and McDonald Islands (Australian territory)
        ELSE 'X'
    END;

UPDATE public.iso_country
SET
    is_country = false
WHERE
    country_code IN (
        248,
        16,
        660,
        533,
        535,
        74,
        86,
        136,
        162,
        166,
        184,
        531,
        238,
        234,
        254,
        258,
        260,
        292,
        304,
        312,
        316,
        831,
        344,
        833,
        832,
        446,
        474,
        175,
        500,
        540,
        570,
        574,
        580,
        612,
        630,
        638,
        652,
        654,
        663,
        666,
        534,
        239,
        744,
        772,
        796,
        581,
        92,
        850,
        876,
        732,
        334
    )
    AND country_code <> 158;

ALTER TABLE public.iso_country
ALTER COLUMN country_flag
SET
    NOT NULL;
