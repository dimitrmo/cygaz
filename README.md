# Cyprus Gas Prices
Directly from the source https://eforms.eservices.cyprus.gov.cy/MCIT/MCIT/PetroleumPrices

## Endpoints

### Get version

#### Request

`GET /version`
    
    curl -i -H 'Accept: text/plain' http://localhost:8080/version

#### Response

    0.1.0

### Get pricing

#### Request

`GET /prices/:petroleum_type`

|                    | Unlead 95 | Unlead 98 | Diesel heat | Diesel auto | Kerosene |
|--------------------|-----------|-----------|-------------|-------------|----------|
| **petroleum_type** | 1         | 2         | 3           | 4           | 5        |


    curl -i -H 'Accept: application/json' http://localhost:8080/prices/4

#### Response

    {
        "updated_at":1647710214169,
        "petroleum_type": 4,
        "stations":[{
            "brand": "Brand1",
            "offline": false,
            "company": "Some company TD",
            "address": "Some address",
            "latitude": "30.0000",
            "longitude":"30.0000",
            "area": "Strovolos",
            "price": 1.000
        }, ...]
    }