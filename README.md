# Cyprus Gas Prices

Directly from the source https://eforms.eservices.cyprus.gov.cy/MCIT/MCIT/PetroleumPrices

## Environment parameters

`TIMEOUT=600000 HOST=0.0.0.0 PORT=8080 ./cygaz`

### Timeout

Timeout in milliseconds

`TIMEOUT=600000`

### Host

Address host

`HOST=0.0.0.0`

### PORT

Address port

`PORT=8080`

## Endpoints

### Get version

#### Request

`GET /version`

    curl -i -H 'Accept: text/plain' http://localhost:8080/version

#### Response

    0.1.3

### Get pricing

#### Request

`GET /prices/:petroleum_type`

|                    | Unlead 95 | Unlead 98 | Diesel heat | Diesel auto | Kerosene |
|--------------------|-----------|-----------|-------------|-------------|----------|
| **petroleum_type** | 1         | 2         | 3           | 4           | 5        |


    curl -i -H 'Accept: application/json' http://localhost:8080/prices/4

#### Response

    {
        "updated_at": 1647710214169,
        "petroleum_type": "DieselAuto",
        "stations": [{
            "brand": "Brand_1",
            "offline": false,
            "company": "Some company TD",
            "address": "Some address",
            "latitude": "30.0000",
            "longitude": "30.0000",
            "area": "Strovolos",
            "price": 1.000,
            "district": {
                "name": "Nicosia",
                "name_el": "Λευκωσία"
            }
        }, ...]
    }